use anyhow::Result;
use std::path::PathBuf;

#[derive(Clone)] // FIXED: Added Clone
pub struct RuntimeInstaller {
    base_dir: PathBuf,
}

impl RuntimeInstaller {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub async fn install(&self, archive: &PathBuf, dest: &PathBuf) -> Result<()> {
        println!("📦 Installing to {}...", dest.display());

        std::fs::create_dir_all(dest)?;

        let extension = archive
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("Unknown archive type"))?;

        match extension {
            "zip" => self.extract_zip(archive, dest)?,
            "gz" | "tgz" => self.extract_tar_gz(archive, dest)?,
            "xz" => self.extract_tar_xz(archive, dest)?,
            _ => anyhow::bail!("Unsupported archive type: {}", extension),
        }

        println!("✅ Installation complete");

        Ok(())
    }

    fn extract_zip(&self, archive: &PathBuf, dest: &PathBuf) -> Result<()> {
        use std::fs::File;
        use zip::ZipArchive;

        let file = File::open(archive)?;
        let mut zip = ZipArchive::new(file)?;

        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let outpath = dest.join(file.name());

            if file.is_dir() {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }

            // Set permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                }
            }
        }

        Ok(())
    }

    fn extract_tar_gz(&self, archive: &PathBuf, dest: &PathBuf) -> Result<()> {
        use flate2::read::GzDecoder;
        use std::fs::File;
        use tar::Archive;

        let file = File::open(archive)?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);

        archive.unpack(dest)?;

        Ok(())
    }

    fn extract_tar_xz(&self, archive: &PathBuf, dest: &PathBuf) -> Result<()> {
        use std::fs::File;
        use tar::Archive;
        use xz2::read::XzDecoder;

        let file = File::open(archive)?;
        let xz = XzDecoder::new(file);
        let mut archive = Archive::new(xz);

        archive.unpack(dest)?;

        Ok(())
    }
}
