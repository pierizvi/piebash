use anyhow::Result;
use colored::*;
use std::fs;
use std::path::Path;

use crate::shell::parser::Command;
use crate::shell::environment::Environment;

pub fn ls(command: &Command, env: &Environment) -> Result<()> {
    // Parse flags and path separately
    let mut show_all = false;
    let mut long_format = false;
    let mut human_readable = false;
    let mut target_path = None;

    for arg in &command.args {
        if arg.starts_with('-') {
            // It's a flag - parse each character
            for ch in arg.chars().skip(1) {
                match ch {
                    'a' => show_all = true,
                    'l' => long_format = true,
                    'h' => human_readable = true,
                    's' => {} // size - ignore for now
                    _ => {}
                }
            }
        } else {
            // It's a path
            target_path = Some(arg.as_str());
        }
    }

    let path = if let Some(p) = target_path {
        env.get_cwd().join(p)
    } else {
        env.get_cwd().clone()
    };

    if !path.exists() {
        anyhow::bail!("ls: cannot access '{}': No such file or directory", path.display());
    }

    if path.is_file() {
        println!("{}", path.display());
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&path)? {
        entries.push(entry?);
    }
    entries.sort_by_key(|e| e.file_name());

    if long_format {
        // Print total
        println!("total {}", entries.len());
        for entry in &entries {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if !show_all && name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata()?;
            let size = metadata.len();
            let is_dir = metadata.is_dir();

            let file_type = if is_dir { "d" } else { "-" };
            let permissions = if is_dir { "rwxr-xr-x" } else { "rw-r--r--" };

            let size_str = if human_readable {
                format_size(size)
            } else {
                size.to_string()
            };

            let modified = metadata.modified()
                .ok()
                .and_then(|t| {
                    let dt: chrono::DateTime<chrono::Local> = t.into();
                    Some(dt.format("%b %d %H:%M").to_string())
                })
                .unwrap_or_else(|| "Jan 01 00:00".to_string());

            let display_name = if is_dir {
                name.blue().bold().to_string()
            } else if is_executable(&entry.path()) {
                name.green().bold().to_string()
            } else {
                name.to_string()
            };

            println!(
                "{}{} {:>3} {:>8} {} {}",
                file_type,
                permissions,
                1,
                size_str,
                modified,
                display_name
            );
        }
    } else {
        // Short format
        for entry in &entries {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if !show_all && name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata()?;
            let is_dir = metadata.is_dir();

            if is_dir {
                print!("{}  ", name.blue().bold());
            } else if is_executable(&entry.path()) {
                print!("{}  ", name.green().bold());
            } else {
                print!("{}  ", name);
            }
        }
        println!();
    }

    Ok(())
}

fn format_size(size: u64) -> String {
    if size >= 1024 * 1024 * 1024 {
        format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if size >= 1024 * 1024 {
        format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
    } else if size >= 1024 {
        format!("{:.1}K", size as f64 / 1024.0)
    } else {
        format!("{}B", size)
    }
}

pub fn cat(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("cat: missing file operand");
    }

    let show_line_numbers = command.args.contains(&"-n".to_string());

    for file in &command.args {
        if file.starts_with('-') { continue; }

        let path = Path::new(file.as_str());
        if !path.exists() {
            eprintln!("cat: {}: No such file or directory", file);
            continue;
        }

        let contents = fs::read_to_string(path)?;

        if show_line_numbers {
            for (i, line) in contents.lines().enumerate() {
                println!("{:>6}  {}", i + 1, line);
            }
        } else {
            print!("{}", contents);
        }
    }

    Ok(())
}

pub fn touch(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("touch: missing file operand");
    }

    for file in &command.args {
        if file.starts_with('-') { continue; }
        let path = Path::new(file.as_str());
        if path.exists() {
            let _ = fs::OpenOptions::new().write(true).open(path)?;
        } else {
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
        if dir.starts_with('-') { continue; }
        let path = Path::new(dir.as_str());
        if recursive {
            fs::create_dir_all(path)?;
        } else {
            if path.exists() {
                anyhow::bail!("mkdir: cannot create directory '{}': File exists", dir);
            }
            fs::create_dir(path)?;
        }
    }

    Ok(())
}

pub fn rm(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("rm: missing operand");
    }

    let mut recursive = false;
    let mut force = false;

    for arg in &command.args {
        if arg.starts_with('-') {
            if arg.contains('r') || arg.contains('R') { recursive = true; }
            if arg.contains('f') { force = true; }
        }
    }

    for item in &command.args {
        if item.starts_with('-') { continue; }

        let path = Path::new(item.as_str());

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
                anyhow::bail!("rm: cannot remove '{}': Is a directory (use -r)", item);
            }
        } else {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

pub fn cp(command: &Command) -> Result<()> {
    let mut recursive = false;
    let mut args: Vec<&String> = Vec::new();

    for arg in &command.args {
        if arg.starts_with('-') {
            if arg.contains('r') || arg.contains('R') { recursive = true; }
        } else {
            args.push(arg);
        }
    }

    if args.len() < 2 {
        anyhow::bail!("cp: missing file operand");
    }

    let source = Path::new(args[0].as_str());
    let dest = Path::new(args[1].as_str());

    if !source.exists() {
        anyhow::bail!("cp: cannot stat '{}': No such file or directory", source.display());
    }

    if source.is_dir() {
        if !recursive {
            anyhow::bail!("cp: -r not specified; omitting directory '{}'", source.display());
        }
        copy_dir_all(source, dest)?;
    } else {
        if dest.is_dir() {
            let filename = source.file_name().unwrap();
            fs::copy(source, dest.join(filename))?;
        } else {
            fs::copy(source, dest)?;
        }
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn mv(command: &Command) -> Result<()> {
    let args: Vec<&String> = command.args.iter().filter(|a| !a.starts_with('-')).collect();

    if args.len() < 2 {
        anyhow::bail!("mv: missing file operand");
    }

    let source = Path::new(args[0].as_str());
    let dest = Path::new(args[1].as_str());

    if !source.exists() {
        anyhow::bail!("mv: cannot stat '{}': No such file or directory", source.display());
    }

    if dest.is_dir() {
        let filename = source.file_name().unwrap();
        fs::rename(source, dest.join(filename))?;
    } else {
        fs::rename(source, dest)?;
    }

    Ok(())
}

pub fn ln(command: &Command) -> Result<()> {
    let symbolic = command.args.contains(&"-s".to_string());
    let args: Vec<&String> = command.args.iter().filter(|a| !a.starts_with('-')).collect();

    if args.len() < 2 {
        anyhow::bail!("ln: missing file operand");
    }

    let source = Path::new(args[0].as_str());
    let dest = Path::new(args[1].as_str());

    #[cfg(unix)]
    if symbolic {
        std::os::unix::fs::symlink(source, dest)?;
    } else {
        fs::hard_link(source, dest)?;
    }

    #[cfg(windows)]
    if symbolic {
        if source.is_dir() {
            std::os::windows::fs::symlink_dir(source, dest)?;
        } else {
            std::os::windows::fs::symlink_file(source, dest)?;
        }
    } else {
        fs::hard_link(source, dest)?;
    }

    Ok(())
}

pub fn chmod(command: &Command) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let args: Vec<&String> = command.args.iter().filter(|a| !a.starts_with('-')).collect();

        if args.len() < 2 {
            anyhow::bail!("chmod: missing operand");
        }

        let mode = u32::from_str_radix(args[0], 8)
            .map_err(|_| anyhow::anyhow!("chmod: invalid mode: {}", args[0]))?;

        for file in &args[1..] {
            let path = Path::new(file.as_str());
            if !path.exists() {
                eprintln!("chmod: cannot access '{}': No such file or directory", file);
                continue;
            }
            fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
        }
    }

    #[cfg(windows)]
    {
        eprintln!("chmod: not fully supported on Windows");
    }

    Ok(())
}

pub fn chown(command: &Command) -> Result<()> {
    #[cfg(unix)]
    {
        eprintln!("chown: not yet implemented");
    }

    #[cfg(windows)]
    {
        eprintln!("chown: not supported on Windows");
    }

    Ok(())
}

pub fn stat(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("stat: missing file operand");
    }

    for file in &command.args {
        if file.starts_with('-') { continue; }

        let path = Path::new(file.as_str());
        if !path.exists() {
            eprintln!("stat: cannot stat '{}': No such file or directory", file);
            continue;
        }

        let metadata = fs::metadata(path)?;
        println!("  File: {}", file);
        println!("  Size: {}", metadata.len());
        println!("  Type: {}", if metadata.is_dir() { "directory" } else { "regular file" });

        if let Ok(modified) = metadata.modified() {
            let dt: chrono::DateTime<chrono::Local> = modified.into();
            println!("Modify: {}", dt.format("%Y-%m-%d %H:%M:%S"));
        }
        if let Ok(created) = metadata.created() {
            let dt: chrono::DateTime<chrono::Local> = created.into();
            println!(" Birth: {}", dt.format("%Y-%m-%d %H:%M:%S"));
        }
    }

    Ok(())
}

pub fn file_cmd(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("file: missing operand");
    }

    for file in &command.args {
        let path = Path::new(file.as_str());
        if !path.exists() {
            eprintln!("file: {}: No such file or directory", file);
            continue;
        }

        let metadata = fs::metadata(path)?;
        if metadata.is_dir() {
            println!("{}: directory", file);
        } else {
            // Read first bytes to guess type
            let bytes = fs::read(path).unwrap_or_default();
            let kind = if bytes.starts_with(b"\x7fELF") {
                "ELF executable"
            } else if bytes.starts_with(b"MZ") {
                "PE32 executable (Windows)"
            } else if bytes.starts_with(b"\x89PNG") {
                "PNG image"
            } else if bytes.starts_with(b"\xff\xd8") {
                "JPEG image"
            } else if bytes.starts_with(b"PK") {
                "Zip archive"
            } else if bytes.starts_with(b"%PDF") {
                "PDF document"
            } else if bytes.is_ascii() {
                "ASCII text"
            } else {
                "binary data"
            };
            println!("{}: {}", file, kind);
        }
    }

    Ok(())
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            return metadata.permissions().mode() & 0o111 != 0;
        }
        false
    }

    #[cfg(windows)]
    {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            return ext == "exe" || ext == "bat" || ext == "cmd" || ext == "com";
        }
        false
    }

    #[cfg(not(any(unix, windows)))]
    false
}