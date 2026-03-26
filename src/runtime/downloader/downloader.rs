use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
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
        let filename = url
            .split('/')
            .last()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL"))?;

        let dest = self.cache_dir.join(filename);

        if dest.exists() {
            println!("Using cached file");
            if self.verify_checksum(&dest, expected_sha)? && self.verify_archive_file(&dest)? {
                return Ok(dest);
            }

            println!("Cached file is invalid, re-downloading");
            std::fs::remove_file(&dest)?;
        }

        println!("Downloading from {}...", url);

        let response = reqwest::get(url).await?.error_for_status()?;
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
                print!(
                    "\rDownload progress: {:.1}% ({} / {} MB)",
                    progress,
                    downloaded / 1024 / 1024,
                    total_size / 1024 / 1024
                );
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }

        println!();

        if !self.verify_checksum(&dest, expected_sha)? {
            std::fs::remove_file(&dest)?;
            anyhow::bail!("Checksum verification failed");
        }

        if !self.verify_archive_file(&dest)? {
            std::fs::remove_file(&dest)?;
            anyhow::bail!("Downloaded file is not a valid archive");
        }

        Ok(dest)
    }

    fn verify_checksum(&self, file: &PathBuf, expected: &str) -> Result<bool> {
        if expected.is_empty() {
            return Ok(true);
        }

        let contents = std::fs::read(file)?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let result = hasher.finalize();
        let hash = format!("{:x}", result);

        Ok(hash == expected)
    }

    fn verify_archive_file(&self, file: &PathBuf) -> Result<bool> {
        use std::fs::File;
        use std::io::Read;

        let metadata = std::fs::metadata(file)?;
        if metadata.len() < 1024 {
            return Ok(false);
        }

        let mut fh = File::open(file)?;
        let mut head = [0u8; 256];
        let n = fh.read(&mut head)?;
        let header_text = String::from_utf8_lossy(&head[..n]).to_ascii_lowercase();

        if header_text.contains("not found") || header_text.contains("<html") {
            return Ok(false);
        }

        Ok(true)
    }
}
