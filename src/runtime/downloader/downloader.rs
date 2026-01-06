use anyhow::Result;
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use tokio::io::AsyncWriteExt;

#[derive(Clone)]  // FIXED: Added Clone
pub struct RuntimeDownloader {
    cache_dir: PathBuf,
}

impl RuntimeDownloader {
    pub fn new(base_dir: PathBuf) -> Self {
        let cache_dir = base_dir.join("cache");
        std::fs::create_dir_all(&cache_dir).ok();

        Self { cache_dir }
    }

    pub async fn download(&self, url: &str, expected_sha: &str) -> Result<PathBuf> {
        let filename = url.split('/').last()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL"))?;

        let dest = self.cache_dir.join(filename);

        // Check if already downloaded
        if dest.exists() {
            println!("📦 Using cached file");
            if self.verify_checksum(&dest, expected_sha)? {
                return Ok(dest);
            } else {
                println!("⚠️  Cached file corrupted, re-downloading");
                std::fs::remove_file(&dest)?;
            }
        }

        // Download
        println!("📥 Downloading from {}...", url);
        
        let response = reqwest::get(url).await?;
        let total_size = response.content_length().unwrap_or(0);

        let mut file = tokio::fs::File::create(&dest).await?;
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let progress = (downloaded as f64 / total_size as f64) * 100.0;
                print!("\r📥 Progress: {:.1}% ({} / {} MB)", 
                    progress,
                    downloaded / 1024 / 1024,
                    total_size / 1024 / 1024
                );
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }

        println!();

        // Verify checksum
        if !self.verify_checksum(&dest, expected_sha)? {
            std::fs::remove_file(&dest)?;
            anyhow::bail!("Checksum verification failed");
        }

        Ok(dest)
    }

    fn verify_checksum(&self, file: &PathBuf, expected: &str) -> Result<bool> {
        if expected.is_empty() {
            return Ok(true); // Skip verification if no checksum provided
        }

        let contents = std::fs::read(file)?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let result = hasher.finalize();
        let hash = format!("{:x}", result);

        Ok(hash == expected)
    }
}