use anyhow::Result;
use colored::*;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

use crate::executor::dependency_detector::{DependencyDetector, MissingDependency};
use crate::runtime::RuntimeManager;
use crate::shell::parser::Command as ShellCommand;

#[derive(Debug, Clone)]
struct FileExecutionContext {
    file: PathBuf,
    working_dir: Option<PathBuf>,
}

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
        let (env_path, executable) = if language == "python" {
            let bootstrap_executable =
                self.python_base_executable(&runtime.path, &runtime.executable);
            let env_path = self
                .ensure_python_env(&runtime.path, &bootstrap_executable)
                .await?;
            let executable = self.python_env_executable(&env_path);
            (env_path, executable)
        } else {
            (runtime.path.clone(), runtime.executable.clone())
        };

        // Track installed packages to avoid loops
        let mut installed_packages: HashSet<String> = HashSet::new();
        let mut attempt = 0;
        let mut last_error_package: Option<String> = None;
        let mut stuck_count = 0;

        loop {
            attempt += 1;

            let file_context = if !command.name.starts_with('@') && !command.args.is_empty() {
                Some(self.prepare_file_execution(language, &env_path, &command.args[0])?)
            } else {
                None
            };

            if attempt > 1 {
                println!("\n{} Retry attempt {}...", "[RETRY]".yellow(), attempt);
            }

            let result = if command.name.starts_with('@') {
                let code = command.args.join(" ");
                self.execute_inline(&executable, &env_path, language, &code)
                    .await
            } else if let Some(file_context) = file_context.as_ref() {
                let args = &command.args[1..];
                self.execute_file(&executable, &env_path, language, file_context, args)
                    .await
            } else {
                anyhow::bail!("No code to execute");
            };

            match result {
                Ok(_) => {
                    // Success! Code ran without errors
                    if attempt > 1 {
                        println!(
                            "\n{} Execution successful after installing {} dependencies",
                            "[SUCCESS]".green().bold(),
                            installed_packages.len()
                        );
                    }
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = e.to_string();

                    // Try to detect and install missing dependencies
                    if let Some(deps) = DependencyDetector::parse_error(language, &error_msg, "") {
                        let mut any_new = false;

                        for dep in &deps {
                            // Check if we're stuck on the same package
                            if let Some(ref last_pkg) = last_error_package {
                                if last_pkg == &dep.package {
                                    stuck_count += 1;
                                    if stuck_count >= 2 {
                                        println!(
                                            "\n{} Unable to install {} after multiple attempts",
                                            "[FAILED]".red().bold(),
                                            dep.package
                                        );
                                        return Err(e);
                                    }
                                } else {
                                    stuck_count = 0;
                                }
                            }
                            last_error_package = Some(dep.package.clone());

                            // Skip if already installed
                            if installed_packages.contains(&dep.package) {
                                println!(
                                    "{} Skipping {} (already installed)",
                                    "[SKIP]".yellow(),
                                    dep.package
                                );
                                continue;
                            }

                            any_new = true;

                            match self
                                .auto_install_dependency(
                                    dep,
                                    &env_path,
                                    &executable,
                                    file_context
                                        .as_ref()
                                        .and_then(|context| context.working_dir.as_ref()),
                                )
                                .await
                            {
                                Ok(_) => {
                                    installed_packages.insert(dep.package.clone());
                                }
                                Err(install_err) => {
                                    eprintln!(
                                        "{} Failed to install {}: {}",
                                        "[ERROR]".red(),
                                        dep.package,
                                        install_err
                                    );
                                }
                            }
                        }

                        if !any_new {
                            // No new packages to install, but still failing
                            stuck_count += 1;
                            if stuck_count >= 2 {
                                println!(
                                    "\n{} No new dependencies detected but still failing",
                                    "[FAILED]".red().bold()
                                );
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

    async fn ensure_python_env(
        &self,
        runtime_path: &PathBuf,
        runtime_executable: &PathBuf,
    ) -> Result<PathBuf> {
        let env_path = runtime_path.join("piebash_env");
        let env_python = self.python_env_executable(&env_path);
        let env_marker = env_path.join("pyvenv.cfg");

        if !env_marker.exists() || !env_python.exists() {
            println!(
                "{} Creating isolated Python environment...",
                "[ENV]".cyan().bold()
            );

            if env_path.exists() {
                println!(
                    "{} Recreating incomplete Python environment",
                    "[FIX]".yellow().bold()
                );
                std::fs::remove_dir_all(&env_path)?;
            }

            let mut cmd = Command::new(runtime_executable);
            cmd.arg("-m");
            cmd.arg("venv");
            cmd.arg(&env_path);
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let output = cmd.output().await?;
            if !output.status.success() {
                let stdout_text = String::from_utf8_lossy(&output.stdout);
                let stderr_text = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!(
                    "Failed to create Python environment\nSTDERR:\n{}\nSTDOUT:\n{}",
                    stderr_text,
                    stdout_text
                );
            }

            if !env_marker.exists() || !env_python.exists() {
                anyhow::bail!(
                    "Python environment creation completed but the virtual environment is incomplete"
                );
            }

            println!("{} Python environment ready", "[OK]".green().bold());
        }

        Ok(env_path)
    }

    fn prepare_file_execution(
        &self,
        language: &str,
        env_path: &PathBuf,
        file: &str,
    ) -> Result<FileExecutionContext> {
        let file_path = Path::new(file);
        if !file_path.exists() {
            anyhow::bail!("File not found: {}", file);
        }

        match language {
            "go" => self.prepare_go_file_context(env_path, file_path),
            _ => Ok(FileExecutionContext {
                file: file_path.to_path_buf(),
                working_dir: None,
            }),
        }
    }

    fn prepare_go_file_context(
        &self,
        env_path: &PathBuf,
        file_path: &Path,
    ) -> Result<FileExecutionContext> {
        let source = fs::read_to_string(file_path)?;
        let canonical = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.to_path_buf());

        let mut digest = Sha256::new();
        digest.update(canonical.to_string_lossy().as_bytes());
        let digest = digest.finalize();
        let workspace_id = digest[..8]
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();

        let workspace = env_path.join("go-workspaces").join(workspace_id);
        fs::create_dir_all(&workspace)?;
        fs::create_dir_all(env_path.join("pkg").join("mod"))?;
        fs::create_dir_all(env_path.join("go-build"))?;

        let go_file = workspace.join("main.go");
        fs::write(&go_file, source)?;

        let go_mod = workspace.join("go.mod");
        if !go_mod.exists() {
            fs::write(&go_mod, "module piebashrun\n\ngo 1.21.5\n")?;
        }

        Ok(FileExecutionContext {
            file: go_file,
            working_dir: Some(workspace),
        })
    }

    async fn execute_inline(
        &self,
        executable: &PathBuf,
        env_path: &PathBuf,
        language: &str,
        code: &str,
    ) -> Result<()> {
        println!("{} Executing inline code...\n", "[RUN]".cyan());

        let mut cmd = Command::new(executable);
        cmd.arg("-c");
        cmd.arg(code);

        self.set_runtime_env(&mut cmd, env_path, language);

        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await?;
        let stdout_text = String::from_utf8_lossy(&output.stdout);
        let stderr_text = String::from_utf8_lossy(&output.stderr);

        if !stdout_text.is_empty() {
            print!("{}", stdout_text);
        }
        if !stderr_text.is_empty() {
            eprint!("{}", stderr_text);
        }

        if !output.status.success() {
            anyhow::bail!(
                "Execution failed with exit code: {:?}\nSTDERR:\n{}\nSTDOUT:\n{}",
                output.status.code(),
                stderr_text,
                stdout_text
            );
        }

        Ok(())
    }

    async fn execute_file(
        &self,
        executable: &PathBuf,
        env_path: &PathBuf,
        language: &str,
        file_context: &FileExecutionContext,
        args: &[String],
    ) -> Result<()> {
        println!(
            "{} Executing {}...\n",
            "[RUN]".cyan(),
            file_context.file.display()
        );

        let mut cmd = Command::new(executable);
        match language {
            "go" => {
                cmd.arg("run");
            }
            "java" => {
                if let Some(classpath) = self.java_classpath(env_path) {
                    cmd.arg("-cp");
                    cmd.arg(classpath);
                }
            }
            _ => {}
        }
        cmd.arg(&file_context.file);
        cmd.args(args);
        if let Some(working_dir) = &file_context.working_dir {
            cmd.current_dir(working_dir);
        }

        self.set_runtime_env(&mut cmd, env_path, language);

        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await?;
        let stdout_text = String::from_utf8_lossy(&output.stdout);
        let stderr_text = String::from_utf8_lossy(&output.stderr);

        if !stdout_text.is_empty() {
            print!("{}", stdout_text);
        }
        if !stderr_text.is_empty() {
            eprint!("{}", stderr_text);
        }

        if !output.status.success() {
            anyhow::bail!(
                "Execution failed with exit code: {:?}\nSTDERR:\n{}\nSTDOUT:\n{}",
                output.status.code(),
                stderr_text,
                stdout_text
            );
        }

        Ok(())
    }

    fn set_runtime_env(&self, cmd: &mut Command, env_path: &PathBuf, language: &str) {
        match language {
            "python" => {
                let scripts_dir = if cfg!(windows) {
                    env_path.join("Scripts")
                } else {
                    env_path.join("bin")
                };

                if scripts_dir.exists() {
                    let current_path = std::env::var("PATH").unwrap_or_default();
                    let separator = if cfg!(windows) { ";" } else { ":" };
                    let new_path = if current_path.is_empty() {
                        scripts_dir.to_string_lossy().to_string()
                    } else {
                        format!("{}{}{}", scripts_dir.display(), separator, current_path)
                    };
                    cmd.env("PATH", new_path);
                }

                cmd.env("VIRTUAL_ENV", env_path);
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
                cmd.env("GOMODCACHE", env_path.join("pkg").join("mod"));
                cmd.env("GOCACHE", env_path.join("go-build"));
            }
            _ => {}
        }
    }

    async fn auto_install_dependency(
        &self,
        dep: &MissingDependency,
        env_path: &PathBuf,
        runtime_executable: &PathBuf,
        execution_dir: Option<&PathBuf>,
    ) -> Result<()> {
        println!(
            "\n{} Missing dependency: {}",
            "[AUTO-INSTALL]".magenta().bold(),
            dep.package.green()
        );
        let manager_label = format!("[{}]", dep.package_manager.to_uppercase());
        println!("{} Installing {}...", manager_label.cyan(), dep.package);

        match dep.language.as_str() {
            "python" => {
                self.install_python_package(dep, env_path, runtime_executable)
                    .await
            }
            "node" => {
                self.install_node_package(dep, env_path, runtime_executable)
                    .await
            }
            "ruby" => self.install_ruby_package(dep, env_path).await,
            "go" => {
                self.install_go_package(dep, env_path, runtime_executable, execution_dir)
                    .await
            }
            "java" => self.install_java_package(dep, env_path).await,
            _ => anyhow::bail!("Package installation not supported for {}", dep.language),
        }
    }

    async fn install_python_package(
        &self,
        dep: &MissingDependency,
        _env_path: &PathBuf,
        python_exe: &PathBuf,
    ) -> Result<()> {
        self.ensure_pip(python_exe).await?;

        let mut cmd = Command::new(python_exe);
        cmd.arg("-m");
        cmd.arg("pip");
        cmd.arg("install");
        cmd.arg("--upgrade");
        cmd.arg("--quiet"); // Less verbose output
        cmd.arg(&dep.package);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("pip install failed for {}", dep.package);
        }

        println!(
            "{} Installed {}",
            "[OK]".green().bold(),
            dep.package.green()
        );
        Ok(())
    }

    async fn ensure_pip(&self, python_exe: &PathBuf) -> Result<()> {
        let mut check = Command::new(python_exe);
        check.arg("-m");
        check.arg("pip");
        check.arg("--version");
        check.stdout(Stdio::null());
        check.stderr(Stdio::null());

        if check
            .status()
            .await
            .ok()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Ok(());
        }

        println!("{} Bootstrapping pip...", "[BOOTSTRAP]".yellow().bold());

        let mut cmd = Command::new(python_exe);
        cmd.arg("-m");
        cmd.arg("ensurepip");
        cmd.arg("--upgrade");
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("Failed to bootstrap pip");
        }

        println!("{} pip ready", "[OK]".green().bold());
        Ok(())
    }

    async fn install_node_package(
        &self,
        dep: &MissingDependency,
        env_path: &PathBuf,
        runtime_executable: &PathBuf,
    ) -> Result<()> {
        let npm_path = self
            .find_runtime_binary(runtime_executable, env_path, "npm")
            .ok_or_else(|| anyhow::anyhow!("npm not found"))?;

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

        println!(
            "{} Installed {}",
            "[OK]".green().bold(),
            dep.package.green()
        );
        Ok(())
    }

    async fn install_ruby_package(
        &self,
        dep: &MissingDependency,
        env_path: &PathBuf,
    ) -> Result<()> {
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

        println!(
            "{} Installed {}",
            "[OK]".green().bold(),
            dep.package.green()
        );
        Ok(())
    }

    async fn install_go_package(
        &self,
        dep: &MissingDependency,
        env_path: &PathBuf,
        runtime_executable: &PathBuf,
        execution_dir: Option<&PathBuf>,
    ) -> Result<()> {
        let go_path = self
            .find_runtime_binary(runtime_executable, env_path, "go")
            .ok_or_else(|| anyhow::anyhow!("go not found"))?;
        let execution_dir = execution_dir
            .ok_or_else(|| anyhow::anyhow!("Go dependency installation requires a workspace"))?;

        if !go_path.exists() {
            anyhow::bail!("go not found");
        }

        let mut cmd = Command::new(&go_path);
        cmd.arg("get");
        cmd.arg(&dep.package);
        cmd.current_dir(execution_dir);
        cmd.env("GOPATH", env_path);
        cmd.env("GOMODCACHE", env_path.join("pkg").join("mod"));
        cmd.env("GOCACHE", env_path.join("go-build"));
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("go get failed for {}", dep.package);
        }

        println!(
            "{} Installed {}",
            "[OK]".green().bold(),
            dep.package.green()
        );
        Ok(())
    }

    async fn install_java_package(
        &self,
        dep: &MissingDependency,
        env_path: &PathBuf,
    ) -> Result<()> {
        let coordinates: Vec<&str> = dep.package.split(':').collect();
        if coordinates.len() != 3 {
            anyhow::bail!("Unsupported Maven coordinate: {}", dep.package);
        }

        let group = coordinates[0];
        let artifact = coordinates[1];
        let version = coordinates[2];

        if version.eq_ignore_ascii_case("LATEST") {
            anyhow::bail!(
                "Dynamic Maven versions are not supported for {}",
                dep.package
            );
        }

        let library_dir = env_path.join("java-libs");
        fs::create_dir_all(&library_dir)?;

        let jar_name = format!("{}-{}.jar", artifact, version);
        let jar_path = library_dir.join(&jar_name);
        if jar_path.exists() {
            println!(
                "{} Installed {}",
                "[OK]".green().bold(),
                dep.package.green()
            );
            return Ok(());
        }

        let jar_url = format!(
            "https://repo1.maven.org/maven2/{}/{}/{}/{}",
            group.replace('.', "/"),
            artifact,
            version,
            jar_name
        );

        let response = reqwest::get(&jar_url).await?.error_for_status()?;
        let content = response.bytes().await?;
        fs::write(&jar_path, &content)?;

        println!(
            "{} Installed {}",
            "[OK]".green().bold(),
            dep.package.green()
        );
        Ok(())
    }

    fn java_classpath(&self, env_path: &PathBuf) -> Option<String> {
        let library_dir = env_path.join("java-libs");
        if !library_dir.is_dir() {
            return None;
        }

        let has_jar = fs::read_dir(&library_dir)
            .ok()?
            .filter_map(|entry| entry.ok())
            .any(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("jar"))
                    .unwrap_or(false)
            });

        if has_jar {
            Some(format!(
                "{}{}*",
                library_dir.display(),
                std::path::MAIN_SEPARATOR
            ))
        } else {
            None
        }
    }

    fn find_runtime_binary(
        &self,
        runtime_executable: &PathBuf,
        runtime_root: &PathBuf,
        name: &str,
    ) -> Option<PathBuf> {
        #[cfg(windows)]
        let names = [
            format!("{}.cmd", name),
            format!("{}.exe", name),
            name.to_string(),
        ];
        #[cfg(not(windows))]
        let names = [name.to_string()];

        if let Some(parent) = runtime_executable.parent() {
            for candidate_name in &names {
                let candidate = parent.join(candidate_name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        for candidate_name in &names {
            let candidate = runtime_root.join(candidate_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        for entry in walkdir::WalkDir::new(runtime_root)
            .max_depth(6)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            for candidate_name in &names {
                if file_name == candidate_name.to_ascii_lowercase() {
                    return Some(entry.path().to_path_buf());
                }
            }
        }

        None
    }

    fn python_env_executable(&self, env_path: &Path) -> PathBuf {
        if cfg!(windows) {
            env_path.join("Scripts").join("python.exe")
        } else {
            env_path.join("bin").join("python")
        }
    }

    fn python_base_executable(&self, runtime_path: &Path, runtime_executable: &Path) -> PathBuf {
        let base = if cfg!(windows) {
            runtime_path.join("python").join("python.exe")
        } else {
            runtime_path.join("bin").join("python")
        };

        if base.is_file() {
            return base;
        }

        let env_root = runtime_path.join("piebash_env");
        if runtime_executable.starts_with(&env_root) {
            return self.find_runtime_binary(
                &runtime_executable.to_path_buf(),
                &runtime_path.to_path_buf(),
                "python",
            )
            .filter(|candidate| !candidate.starts_with(&env_root))
            .unwrap_or_else(|| runtime_executable.to_path_buf());
        }

        runtime_executable.to_path_buf()
    }
}
