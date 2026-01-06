use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageRegistry {
    languages: HashMap<String, LanguageDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDefinition {
    pub name: String,
    pub version: String,
    pub executable: String,
    pub downloads: HashMap<String, DownloadInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub url: String,
    pub sha256: String,
}

impl LanguageRegistry {
    pub fn load() -> Result<Self> {
        // For now, return hardcoded registry
        // Later: load from data/registry/languages.json
        Ok(Self::default())
    }

    pub fn get_language(&self, name: &str) -> Result<LanguageDefinition> {
        self.languages
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Language not found: {}", name))
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        let mut languages = HashMap::new();

        // Python
        languages.insert("python".to_string(), LanguageDefinition {
            name: "Python".to_string(),
            version: "3.11.6".to_string(),
            executable: "python".to_string(),
            downloads: {
                let mut map = HashMap::new();
                map.insert("linux-x86_64".to_string(), DownloadInfo {
                    url: "https://github.com/indygreg/python-build-standalone/releases/download/20231002/cpython-3.11.6+20231002-x86_64-unknown-linux-gnu-install_only.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("windows-x86_64".to_string(), DownloadInfo {
                    url: "https://www.python.org/ftp/python/3.11.6/python-3.11.6-embed-amd64.zip".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("darwin-x86_64".to_string(), DownloadInfo {
                    url: "https://github.com/indygreg/python-build-standalone/releases/download/20231002/cpython-3.11.6+20231002-x86_64-apple-darwin-install_only.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map
            },
        });

        // Node.js
        languages.insert("node".to_string(), LanguageDefinition {
            name: "Node.js".to_string(),
            version: "20.10.0".to_string(),
            executable: "node".to_string(),
            downloads: {
                let mut map = HashMap::new();
                map.insert("linux-x86_64".to_string(), DownloadInfo {
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-linux-x64.tar.xz".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("windows-x86_64".to_string(), DownloadInfo {
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-win-x64.zip".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("darwin-x86_64".to_string(), DownloadInfo {
                    url: "https://nodejs.org/dist/v20.10.0/node-v20.10.0-darwin-x64.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map
            },
        });

        Self { languages }
    }
}

impl LanguageDefinition {
    pub fn get_download_url(&self, platform: &str) -> Result<DownloadInfo> {
        self.downloads
            .get(platform)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No download available for platform: {}", platform))
    }
}