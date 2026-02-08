use anyhow::Result;
use colored::*;
use std::fs;
use std::path::Path;

use crate::shell::parser::Command;
use crate::shell::environment::Environment;

pub fn ls(command: &Command, env: &Environment) -> Result<()> {
    let path = if command.args.is_empty() {
        env.get_cwd().clone()
    } else {
        env.get_cwd().join(&command.args[0])
    };

    if !path.exists() {
        anyhow::bail!("ls: cannot access '{}': No such file or directory", path.display());
    }

    if path.is_file() {
        println!("{}", path.display());
        return Ok(());
    }

    // List directory contents
    let mut entries = Vec::new();
    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        entries.push(entry);
    }

    // Sort entries
    entries.sort_by_key(|e| e.file_name());

    // Check for -la flag
    let show_all = command.args.contains(&"-a".to_string()) 
        || command.args.contains(&"-la".to_string())
        || command.args.contains(&"-al".to_string());
    
    let long_format = command.args.contains(&"-l".to_string())
        || command.args.contains(&"-la".to_string())
        || command.args.contains(&"-al".to_string());

    for entry in entries {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Skip hidden files unless -a
        if !show_all && name.starts_with('.') {
            continue;
        }

        if long_format {
            let metadata = entry.metadata()?;
            let size = metadata.len();
            let file_type = if metadata.is_dir() { "d" } else { "-" };
            
            let display_name = if metadata.is_dir() {
                name.blue().bold()
            } else if is_executable(&entry.path()) {
                name.green().bold()
            } else {
                name.normal()
            };

            println!("{} {:>10} {}", file_type, size, display_name);
        } else {
            let metadata = entry.metadata()?;
            let display_name = if metadata.is_dir() {
                format!("{}/", name).blue().bold()
            } else if is_executable(&entry.path()) {
                name.green().bold()
            } else {
                name.normal()
            };

            print!("{}  ", display_name);
        }
    }

    if !long_format {
        println!();
    }

    Ok(())
}

pub fn cat(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("cat: missing file operand");
    }

    for file in &command.args {
        let path = Path::new(file);
        if !path.exists() {
            eprintln!("cat: {}: No such file or directory", file);
            continue;
        }

        let contents = fs::read_to_string(path)?;
        print!("{}", contents);
    }

    Ok(())
}

pub fn touch(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("touch: missing file operand");
    }

    for file in &command.args {
        let path = Path::new(file);
        
        if path.exists() {
            // Update timestamp
            let _ = fs::OpenOptions::new().write(true).open(path)?;
        } else {
            // Create new file
            fs::File::create(path)?;
        }
    }

    Ok(())
}

pub fn mkdir(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("mkdir: missing operand");
    }

    let recursive = command.args.contains(&"-p".to_string());

    for dir in &command.args {
        if dir == "-p" {
            continue;
        }

        let path = Path::new(dir);
        
        if recursive {
            fs::create_dir_all(path)?;
        } else {
            fs::create_dir(path)?;
        }
    }

    Ok(())
}

pub fn rm(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("rm: missing operand");
    }

    let recursive = command.args.contains(&"-r".to_string()) 
        || command.args.contains(&"-rf".to_string());
    let force = command.args.contains(&"-f".to_string())
        || command.args.contains(&"-rf".to_string());

    for item in &command.args {
        if item.starts_with('-') {
            continue;
        }

        let path = Path::new(item);

        if !path.exists() {
            if !force {
                eprintln!("rm: cannot remove '{}': No such file or directory", item);
            }
            continue;
        }

        if path.is_dir() {
            if recursive {
                fs::remove_dir_all(path)?;
            } else {
                anyhow::bail!("rm: cannot remove '{}': Is a directory", item);
            }
        } else {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

pub fn cp(command: &Command) -> Result<()> {
    if command.args.len() < 2 {
        anyhow::bail!("cp: missing file operand");
    }

    let source = Path::new(&command.args[0]);
    let dest = Path::new(&command.args[1]);

    if !source.exists() {
        anyhow::bail!("cp: cannot stat '{}': No such file or directory", source.display());
    }

    if source.is_dir() {
        anyhow::bail!("cp: -r not specified; omitting directory '{}'", source.display());
    }

    fs::copy(source, dest)?;

    Ok(())
}

pub fn mv(command: &Command) -> Result<()> {
    if command.args.len() < 2 {
        anyhow::bail!("mv: missing file operand");
    }

    let source = Path::new(&command.args[0]);
    let dest = Path::new(&command.args[1]);

    if !source.exists() {
        anyhow::bail!("mv: cannot stat '{}': No such file or directory", source.display());
    }

    fs::rename(source, dest)?;

    Ok(())
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let permissions = metadata.permissions();
            return permissions.mode() & 0o111 != 0;
        }
        false
    }

    #[cfg(windows)]
    {
        // On Windows, check file extension
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            return ext == "exe" || ext == "bat" || ext == "cmd" || ext == "com";
        }
        false
    }

    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}