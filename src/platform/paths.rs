use std::path::{Path, PathBuf};

pub struct PathConverter;

impl PathConverter {
    pub fn to_linux_style(path: &Path) -> String {
        #[cfg(windows)]
        {
            let path_str = path.to_string_lossy();
            // C:\Users\Name -> /c/Users/Name
            if path_str.len() >= 2 && path_str.chars().nth(1) == Some(':') {
                let drive = path_str.chars().next().unwrap().to_lowercase();
                let rest = &path_str[2..].replace('\\', "/");
                return format!("/{}{}", drive, rest);
            }
            path_str.replace('\\', "/")
        }

        #[cfg(not(windows))]
        {
            path.to_string_lossy().to_string()
        }
    }

    pub fn from_linux_style(path: &str) -> PathBuf {
        #[cfg(windows)]
        {
            // /c/Users/Name -> C:\Users\Name
            if path.starts_with('/') && path.len() > 2 && path.chars().nth(2) == Some('/') {
                let drive = path.chars().nth(1).unwrap().to_uppercase();
                let rest = &path[2..].replace('/', "\\");
                return PathBuf::from(format!("{}:{}", drive, rest));
            }
            PathBuf::from(path.replace('/', "\\"))
        }

        #[cfg(not(windows))]
        {
            PathBuf::from(path)
        }
    }
}