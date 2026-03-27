use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::language::registry::LanguageRegistry;
use crate::runtime::downloader::RuntimeDownloader;
use crate::runtime::installer::RuntimeInstaller;

#[derive(Clone)]
pub struct RuntimeManager {
    base_dir: PathBuf,
    registry: Arc<LanguageRegistry>,
    downloader: RuntimeDownloader,
    installer: RuntimeInstaller,
    installed: Arc<RwLock<HashMap<String, RuntimeInfo>>>,
}

#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    pub language: String,
    pub version: String,
    pub path: PathBuf,
    pub executable: PathBuf,
}

impl RuntimeManager {
    pub async fn new() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        let base_dir = home.join(".piebash");
        std::fs::create_dir_all(&base_dir)?;

        let registry = Arc::new(LanguageRegistry::load()?);
        let downloader = RuntimeDownloader::new(base_dir.clone());
        let installer = RuntimeInstaller::new(base_dir.clone());

        let mut manager = Self {
            base_dir,
            registry,
            downloader,
            installer,
            installed: Arc::new(RwLock::new(HashMap::new())),
        };

        manager.scan_installed().await?;
        Ok(manager)
    }

    pub async fn ensure_runtime(&self, language: &str) -> Result<RuntimeInfo> {
        {
            let installed = self.installed.read().await;
            if let Some(info) = installed.get(language) {
                return Ok(info.clone());
            }
        }

        println!("📦 {} runtime not found", language);
        self.install_runtime(language).await
    }

    async fn install_runtime(&self, language: &str) -> Result<RuntimeInfo> {
        println!("📥 Downloading {}...", language);

        let lang_def = self.registry.get_language(language)?;

        let platform = crate::platform::detect_platform();
        println!("📍 Platform: {}", platform);

        let download_info = lang_def.get_download_url(&platform)?;
        let archive_path = self
            .downloader
            .download(&download_info.url, &download_info.sha256)
            .await?;
        println!("✅ Download complete");

        let runtime_dir = self
            .base_dir
            .join("runtimes")
            .join(format!("{}-{}", language, lang_def.version));

        self.installer.install(&archive_path, &runtime_dir).await?;
        println!(
            "✅ {} {} installed to {}",
            language,
            lang_def.version,
            runtime_dir.display()
        );

        let executable = self.find_executable(&runtime_dir, &lang_def.executable)?;
        self.verify_runtime(language, &executable)?;

        let info = RuntimeInfo {
            language: language.to_string(),
            version: lang_def.version.clone(),
            path: runtime_dir,
            executable,
        };

        {
            let mut installed = self.installed.write().await;
            installed.insert(language.to_string(), info.clone());
        }

        println!("✅ {} ready to use!", language);
        Ok(info)
    }

    async fn scan_installed(&mut self) -> Result<()> {
        let runtimes_dir = self.base_dir.join("runtimes");
        if !runtimes_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(runtimes_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(info) = self.parse_runtime_dir(&entry.path()).await? {
                    let mut installed = self.installed.write().await;
                    installed.insert(info.language.clone(), info);
                }
            }
        }

        Ok(())
    }

    async fn parse_runtime_dir(&self, path: &PathBuf) -> Result<Option<RuntimeInfo>> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid runtime directory name")?;

        let parts: Vec<&str> = name.splitn(2, '-').collect();
        if parts.len() != 2 {
            return Ok(None);
        }

        let language = parts[0].to_string();
        let version = parts[1].to_string();

        let lang_def = match self.registry.get_language(&language) {
            Ok(def) => def,
            Err(_) => return Ok(None),
        };

        let executable = match self.find_executable(path, &lang_def.executable) {
            Ok(exe) => exe,
            Err(_) => return Ok(None),
        };

        Ok(Some(RuntimeInfo {
            language,
            version,
            path: path.clone(),
            executable,
        }))
    }

    fn find_executable(&self, runtime_dir: &PathBuf, exe_name: &str) -> Result<PathBuf> {
        let candidates = vec![
            runtime_dir.join("bin").join(exe_name),
            runtime_dir.join(exe_name),
            #[cfg(windows)]
            runtime_dir.join("bin").join(format!("{}.exe", exe_name)),
            #[cfg(windows)]
            runtime_dir.join(format!("{}.exe", exe_name)),
        ];

        for candidate in candidates {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        #[cfg(windows)]
        let exe_filename = format!("{}.exe", exe_name);
        #[cfg(not(windows))]
        let exe_filename = exe_name.to_string();

        for entry in walkdir::WalkDir::new(runtime_dir)
            .max_depth(6)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy();
            if file_name == exe_name || file_name == exe_filename {
                return Ok(entry.path().to_path_buf());
            }
        }

        anyhow::bail!("Could not find executable: {}", exe_name)
    }

    fn verify_runtime(&self, language: &str, executable: &PathBuf) -> Result<()> {
        use std::process::Command;

        let output = match language {
            "go" => Command::new(executable)
                .arg("version")
                .output()
                .context("Failed to verify runtime")?,
            "java" => {
                let primary = Command::new(executable)
                    .arg("--version")
                    .output()
                    .context("Failed to verify runtime")?;
                if primary.status.success() {
                    primary
                } else {
                    Command::new(executable)
                        .arg("-version")
                        .output()
                        .context("Failed to verify runtime")?
                }
            }
            _ => Command::new(executable)
                .arg("--version")
                .output()
                .context("Failed to verify runtime")?,
        };

        if !output.status.success() {
            anyhow::bail!("Runtime verification failed");
        }

        let version_text = if output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stderr).to_string()
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };
        println!("✓ Verified: {}", version_text.trim());

        Ok(())
    }
}
