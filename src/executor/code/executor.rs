use anyhow::Result;
use tokio::process::Command;
use std::process::Stdio;
use colored::*;
use std::path::PathBuf;
use std::collections::HashSet;

use crate::runtime::RuntimeManager;
use crate::shell::parser::Command as ShellCommand;
use crate::executor::dependency_detector::{DependencyDetector, MissingDependency};

#[derive(Clone)]
pub struct CodeExecutor {
    runtime_manager: RuntimeManager,
    detector: DependencyDetector,
}

impl CodeExecutor {
    pub fn new(runtime_manager: RuntimeManager) -> Self {
        Self {
            runtime_manager,
            detector: DependencyDetector::new(),
        }
    }

    pub async fn execute(&self, language: &str, command: &ShellCommand) -> Result<()> {
        let runtime = self.runtime_manager.ensure_runtime(language).await?;

        // Setup isolated environment
        let env_path = if language == "python" {
            self.ensure_python_env(&runtime.path).await?
        } else {
            runtime.path.clone()
        };

        // Track installed packages to avoid loops
        let mut installed_packages: HashSet<String> = HashSet::new();
        let mut attempt = 0;
        let mut last_error_package: Option<String> = None;
        let mut stuck_count = 0;

        loop {
            attempt += 1;

            if attempt > 1 {
                println!("\n{} Retry attempt {}...", "[RETRY]".yellow(), attempt);
            }

            let result = if command.name.starts_with('@') {
                let code = command.args.join(" ");
                self.execute_inline(&runtime.executable, &env_path, language, &code).await
            } else if !command.args.is_empty() {
                let file = &command.args[0];
                let args = &command.args[1..];
                self.execute_file(&runtime.executable, &env_path, language, file, args).await
            } else {
                anyhow::bail!("No code to execute");
            };

            match result {
                Ok(_) => {
                    // Success! Code ran without errors
                    if attempt > 1 {
                        println!("\n{} Execution successful after installing {} dependencies", 
                            "[SUCCESS]".green().bold(), installed_packages.len());
                    }
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = format!("{:?}", e);
                    
                    // Try to detect and install missing dependencies
                    if let Some(deps) = DependencyDetector::parse_error(language, &error_msg, "") {
                        let mut any_new = false;
                        
                        for dep in &deps {
                            // Check if we're stuck on the same package
                            if let Some(ref last_pkg) = last_error_package {
                                if last_pkg == &dep.package {
                                    stuck_count += 1;
                                    if stuck_count >= 2 {
                                        println!("\n{} Unable to install {} after multiple attempts", 
                                            "[FAILED]".red().bold(), dep.package);
                                        return Err(e);
                                    }
                                } else {
                                    stuck_count = 0;
                                }
                            }
                            last_error_package = Some(dep.package.clone());
                            
                            // Skip if already installed
                            if installed_packages.contains(&dep.package) {
                                println!("{} Skipping {} (already installed)", "[SKIP]".yellow(), dep.package);
                                continue;
                            }
                            
                            any_new = true;
                            
                            match self.auto_install_dependency(dep, &env_path, &runtime.executable).await {
                                Ok(_) => {
                                    installed_packages.insert(dep.package.clone());
                                }
                                Err(install_err) => {
                                    eprintln!("{} Failed to install {}: {}", 
                                        "[ERROR]".red(), dep.package, install_err);
                                }
                            }
                        }
                        
                        if !any_new {
                            // No new packages to install, but still failing
                            stuck_count += 1;
                            if stuck_count >= 2 {
                                println!("\n{} No new dependencies detected but still failing", 
                                    "[FAILED]".red().bold());
                                return Err(e);
                            }
                        }
                        
                        // Continue loop to retry
                        continue;
                    } else {
                        // Not a dependency error - this is a real error
                        return Err(e);
                    }
                }
            }
        }
    }

    async fn ensure_python_env(&self, runtime_path: &PathBuf) -> Result<PathBuf> {
        let env_path = runtime_path.join("piebash_env");
        let site_packages = if cfg!(windows) {
            env_path.join("Lib").join("site-packages")
        } else {
            env_path.join("lib").join("python3.11").join("site-packages")
        };

        if !site_packages.exists() {
            println!("{} Creating isolated environment (like Docker container)...", "[ENV]".cyan().bold());
            std::fs::create_dir_all(&site_packages)?;
            
            let pth_file = if cfg!(windows) {
                runtime_path.join("python311._pth")
            } else {
                site_packages.parent().unwrap().join("sitecustomize.py")
            };
            
            if cfg!(windows) {
                let pth_content = format!(
                    "python311.zip\n.\n\n# Uncomment to run site.main() automatically\nimport site\n{}",
                    site_packages.display().to_string().replace('\\', "/")
                );
                std::fs::write(&pth_file, pth_content)?;
            }
            
            println!("{} Isolated environment created", "[OK]".green().bold());
        }

        Ok(env_path)
    }

    async fn execute_inline(&self, executable: &PathBuf, env_path: &PathBuf, language: &str, code: &str) -> Result<()> {
        println!("{} Executing inline code...\n", "[RUN]".cyan());

        let mut cmd = Command::new(executable);
        cmd.arg("-c");
        cmd.arg(code);
        
        self.set_runtime_env(&mut cmd, env_path, language);
        
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("Execution failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    async fn execute_file(&self, executable: &PathBuf, env_path: &PathBuf, language: &str, file: &str, args: &[String]) -> Result<()> {
        println!("{} Executing {}...\n", "[RUN]".cyan(), file);

        let file_path = std::path::Path::new(file);
        if !file_path.exists() {
            anyhow::bail!("File not found: {}", file);
        }

        let mut cmd = Command::new(executable);
        cmd.arg(file_path);
        cmd.args(args);
        
        self.set_runtime_env(&mut cmd, env_path, language);
        
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("Execution failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    fn set_runtime_env(&self, cmd: &mut Command, env_path: &PathBuf, language: &str) {
        match language {
            "python" => {
                let site_packages = if cfg!(windows) {
                    env_path.join("Lib").join("site-packages")
                } else {
                    env_path.join("lib").join("python3.11").join("site-packages")
                };
                
                if site_packages.exists() {
                    let current_path = std::env::var("PYTHONPATH").unwrap_or_default();
                    let new_path = if current_path.is_empty() {
                        site_packages.to_string_lossy().to_string()
                    } else {
                        format!("{}{}{}", site_packages.display(), if cfg!(windows) { ";" } else { ":" }, current_path)
                    };
                    cmd.env("PYTHONPATH", new_path);
                }
            }
            "node" => {
                let node_modules = env_path.join("node_modules");
                if node_modules.exists() {
                    cmd.env("NODE_PATH", &node_modules);
                }
            }
            "ruby" => {
                let gem_home = env_path.join("gems");
                if gem_home.exists() {
                    cmd.env("GEM_HOME", &gem_home);
                }
            }
            "go" => {
                cmd.env("GOPATH", env_path);
            }
            _ => {}
        }
    }

    async fn auto_install_dependency(&self, dep: &MissingDependency, env_path: &PathBuf, python_exe: &PathBuf) -> Result<()> {
        println!("\n{} Missing dependency: {}", "[AUTO-INSTALL]".magenta().bold(), dep.package.green());
        println!("{} Installing {}...", "[PIP]".cyan(), dep.package);

        match dep.language.as_str() {
            "python" => self.install_python_package(dep, env_path, python_exe).await,
            "node" => self.install_node_package(dep, env_path).await,
            "ruby" => self.install_ruby_package(dep, env_path).await,
            "go" => self.install_go_package(dep, env_path).await,
            _ => anyhow::bail!("Package installation not supported for {}", dep.language),
        }
    }

    async fn install_python_package(&self, dep: &MissingDependency, env_path: &PathBuf, python_exe: &PathBuf) -> Result<()> {
        self.ensure_pip(python_exe, env_path).await?;

        let site_packages = if cfg!(windows) {
            env_path.join("Lib").join("site-packages")
        } else {
            env_path.join("lib").join("python3.11").join("site-packages")
        };

        let mut cmd = Command::new(python_exe);
        cmd.arg("-m");
        cmd.arg("pip");
        cmd.arg("install");
        cmd.arg("--target");
        cmd.arg(&site_packages);
        cmd.arg("--upgrade");
        cmd.arg("--quiet");  // Less verbose output
        cmd.arg(&dep.package);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("pip install failed for {}", dep.package);
        }

        println!("{} Installed {}", "[OK]".green().bold(), dep.package.green());
        Ok(())
    }

    async fn ensure_pip(&self, python_exe: &PathBuf, env_path: &PathBuf) -> Result<()> {
        let mut check = Command::new(python_exe);
        check.arg("-m");
        check.arg("pip");
        check.arg("--version");
        check.stdout(Stdio::null());
        check.stderr(Stdio::null());
        
        let site_packages = if cfg!(windows) {
            env_path.join("Lib").join("site-packages")
        } else {
            env_path.join("lib").join("python3.11").join("site-packages")
        };
        check.env("PYTHONPATH", &site_packages);

        if check.status().await.ok().map(|s| s.success()).unwrap_or(false) {
            return Ok(());
        }

        println!("{} Bootstrapping pip...", "[BOOTSTRAP]".yellow().bold());

        let get_pip_url = "https://bootstrap.pypa.io/get-pip.py";
        let get_pip_path = env_path.join("get-pip.py");

        let response = reqwest::get(get_pip_url).await?;
        let content = response.bytes().await?;
        std::fs::write(&get_pip_path, &content)?;

        let mut cmd = Command::new(python_exe);
        cmd.arg(&get_pip_path);
        cmd.arg("--target");
        cmd.arg(&site_packages);
        cmd.arg("--quiet");
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;
        let _ = std::fs::remove_file(&get_pip_path);

        if !status.success() {
            anyhow::bail!("Failed to bootstrap pip");
        }

        println!("{} pip ready", "[OK]".green().bold());
        Ok(())
    }

    async fn install_node_package(&self, dep: &MissingDependency, env_path: &PathBuf) -> Result<()> {
        let npm_path = if cfg!(windows) {
            env_path.parent().unwrap().join("npm.cmd")
        } else {
            env_path.parent().unwrap().join("bin").join("npm")
        };

        if !npm_path.exists() {
            anyhow::bail!("npm not found");
        }

        let mut cmd = Command::new(&npm_path);
        cmd.arg("install");
        cmd.arg("--prefix");
        cmd.arg(env_path);
        cmd.arg(&dep.package);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("npm install failed for {}", dep.package);
        }

        println!("{} Installed {}", "[OK]".green().bold(), dep.package.green());
        Ok(())
    }

    async fn install_ruby_package(&self, dep: &MissingDependency, env_path: &PathBuf) -> Result<()> {
        let gem_path = if cfg!(windows) {
            env_path.parent().unwrap().join("bin").join("gem.exe")
        } else {
            env_path.parent().unwrap().join("bin").join("gem")
        };

        if !gem_path.exists() {
            anyhow::bail!("gem not found");
        }

        let gem_home = env_path.join("gems");
        std::fs::create_dir_all(&gem_home)?;

        let mut cmd = Command::new(&gem_path);
        cmd.arg("install");
        cmd.arg(&dep.package);
        cmd.arg("--install-dir");
        cmd.arg(&gem_home);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("gem install failed for {}", dep.package);
        }

        println!("{} Installed {}", "[OK]".green().bold(), dep.package.green());
        Ok(())
    }

    async fn install_go_package(&self, dep: &MissingDependency, env_path: &PathBuf) -> Result<()> {
        let go_path = if cfg!(windows) {
            env_path.parent().unwrap().join("bin").join("go.exe")
        } else {
            env_path.parent().unwrap().join("bin").join("go")
        };

        if !go_path.exists() {
            anyhow::bail!("go not found");
        }

        let mut cmd = Command::new(&go_path);
        cmd.arg("get");
        cmd.arg(&dep.package);
        cmd.env("GOPATH", env_path);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("go get failed for {}", dep.package);
        }

        println!("{} Installed {}", "[OK]".green().bold(), dep.package.green());
        Ok(())
    }
}