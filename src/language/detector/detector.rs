use anyhow::Result;
use std::path::Path;

pub struct LanguageDetector {
    extensions: std::collections::HashMap<String, String>,
}

impl LanguageDetector {
    pub fn new() -> Result<Self> {
        let mut extensions = std::collections::HashMap::new();

        // Python
        extensions.insert("py".to_string(), "python".to_string());
        extensions.insert("pyw".to_string(), "python".to_string());

        // JavaScript/Node.js
        extensions.insert("js".to_string(), "node".to_string());
        extensions.insert("mjs".to_string(), "node".to_string());
        extensions.insert("cjs".to_string(), "node".to_string());

        // Java
        extensions.insert("java".to_string(), "java".to_string());

        // Rust
        extensions.insert("rs".to_string(), "rust".to_string());

        // Go
        extensions.insert("go".to_string(), "go".to_string());

        // Ruby
        extensions.insert("rb".to_string(), "ruby".to_string());

        // PHP
        extensions.insert("php".to_string(), "php".to_string());

        // C/C++
        extensions.insert("c".to_string(), "gcc".to_string());
        extensions.insert("cpp".to_string(), "g++".to_string());
        extensions.insert("cc".to_string(), "g++".to_string());

        // Shell
        extensions.insert("sh".to_string(), "bash".to_string());

        // Perl
        extensions.insert("pl".to_string(), "perl".to_string());

        // Lua
        extensions.insert("lua".to_string(), "lua".to_string());

        Ok(Self { extensions })
    }

    pub fn detect_from_file(&self, file: &str) -> Result<String> {
        let path = Path::new(file);
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine file type"))?;

        self.extensions
            .get(extension)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unsupported file type: .{}", extension))
    }
}