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

        // Go
        languages.insert("go".to_string(), LanguageDefinition {
            name: "Go".to_string(),
            version: "1.21.5".to_string(),
            executable: "go".to_string(),
            downloads: {
                let mut map = HashMap::new();
                map.insert("linux-x86_64".to_string(), DownloadInfo {
                    url: "https://go.dev/dl/go1.21.5.linux-amd64.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("windows-x86_64".to_string(), DownloadInfo {
                    url: "https://go.dev/dl/go1.21.5.windows-amd64.zip".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("darwin-x86_64".to_string(), DownloadInfo {
                    url: "https://go.dev/dl/go1.21.5.darwin-amd64.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map
            },
        });

        // Rust
        languages.insert("rust".to_string(), LanguageDefinition {
            name: "Rust".to_string(),
            version: "1.75.0".to_string(),
            executable: "rustc".to_string(),
            downloads: {
                let mut map = HashMap::new();
                map.insert("linux-x86_64".to_string(), DownloadInfo {
                    url: "https://static.rust-lang.org/dist/rust-1.75.0-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("windows-x86_64".to_string(), DownloadInfo {
                    url: "https://static.rust-lang.org/dist/rust-1.75.0-x86_64-pc-windows-msvc.zip".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("darwin-x86_64".to_string(), DownloadInfo {
                    url: "https://static.rust-lang.org/dist/rust-1.75.0-x86_64-apple-darwin.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map
            },
        });

        // Ruby
        languages.insert("ruby".to_string(), LanguageDefinition {
            name: "Ruby".to_string(),
            version: "3.2.2".to_string(),
            executable: "ruby".to_string(),
            downloads: {
                let mut map = HashMap::new();
                map.insert("linux-x86_64".to_string(), DownloadInfo {
                    url: "https://cache.ruby-lang.org/pub/ruby/3.2/ruby-3.2.2.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("windows-x86_64".to_string(), DownloadInfo {
                    url: "https://github.com/oneclick/rubyinstaller2/releases/download/RubyInstaller-3.2.2-1/rubyinstaller-3.2.2-1-x64.zip".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("darwin-x86_64".to_string(), DownloadInfo {
                    url: "https://cache.ruby-lang.org/pub/ruby/3.2/ruby-3.2.2.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map
            },
        });

        // Java (OpenJDK)
        languages.insert("java".to_string(), LanguageDefinition {
            name: "Java".to_string(),
            version: "21.0.1".to_string(),
            executable: "java".to_string(),
            downloads: {
                let mut map = HashMap::new();
                map.insert("linux-x86_64".to_string(), DownloadInfo {
                    url: "https://download.java.net/java/GA/jdk21.0.1/415e3f918a1f4062a0074a2794853d0d/12/GPL/openjdk-21.0.1_linux-x64_bin.tar.gz".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("windows-x86_64".to_string(), DownloadInfo {
                    url: "https://download.java.net/java/GA/jdk21.0.1/415e3f918a1f4062a0074a2794853d0d/12/GPL/openjdk-21.0.1_windows-x64_bin.zip".to_string(),
                    sha256: "".to_string(),
                });
                map.insert("darwin-x86_64".to_string(), DownloadInfo {
                    url: "https://download.java.net/java/GA/jdk21.0.1/415e3f918a1f4062a0074a2794853d0d/12/GPL/openjdk-21.0.1_macos-x64_bin.tar.gz".to_string(),
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