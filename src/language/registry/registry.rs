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
    pub package_manager: Option<PackageManager>,
    pub downloads: HashMap<String, DownloadInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManager {
    pub name: String,
    pub executable: String,
    pub install_cmd: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
struct RegistryFile {
    version: String,
    languages: HashMap<String, LanguageDefinition>,
}

impl LanguageRegistry {
    pub fn load() -> Result<Self> {
        let registry: RegistryFile = serde_json::from_str(include_str!(
            "../../../data/registry/languages.json"
        ))?;

        let _ = registry.version;

        Ok(Self {
            languages: registry.languages,
        })
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
        Self::load().expect("embedded language registry is valid")
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

#[cfg(test)]
mod tests {
    use super::LanguageRegistry;

    #[test]
    fn loads_languages_from_embedded_registry() {
        let registry = LanguageRegistry::load().expect("registry loads");

        assert!(registry.get_language("python").is_ok());
        assert!(registry.get_language("node").is_ok());
        assert!(registry.get_language("rust").is_ok());
    }
}
