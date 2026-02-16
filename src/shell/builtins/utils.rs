use anyhow::Result;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::shell::parser::Command;

pub fn find(command: &Command) -> Result<()> {
    let path = if command.args.is_empty() {
        ".".to_string()
    } else {
        command.args[0].clone()
    };

    let pattern = if command.args.len() > 2 && command.args[1] == "-name" {
        Some(&command.args[2])
    } else {
        None
    };

    let max_depth = if let Some(idx) = command.args.iter().position(|a| a == "-maxdepth") {
        if idx + 1 < command.args.len() {
            command.args[idx + 1].parse::<usize>().ok()
        } else {
            None
        }
    } else {
        None
    };

    let mut walker = WalkDir::new(&path);
    if let Some(depth) = max_depth {
        walker = walker.max_depth(depth);
    }

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        if let Some(pat) = pattern {
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.contains(pat) {
                    println!("{}", path.display());
                }
            }
        } else {
            println!("{}", path.display());
        }
    }

    Ok(())
}

pub fn wc(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("wc: missing file operand");
    }

    let count_lines = command.args.contains(&"-l".to_string());
    let count_words = command.args.contains(&"-w".to_string());
    let count_chars = command.args.contains(&"-c".to_string());
    
    let show_all = !count_lines && !count_words && !count_chars;

    for file in &command.args {
        if file.starts_with('-') {
            continue;
        }

        let path = Path::new(file.as_str());
        if !path.exists() {
            eprintln!("wc: {}: No such file or directory", file);
            continue;
        }

        let contents = fs::read_to_string(path)?;
        let lines = contents.lines().count();
        let words = contents.split_whitespace().count();
        let chars = contents.len();

        if show_all {
            println!("{:>8} {:>8} {:>8} {}", lines, words, chars, file);
        } else {
            let mut parts = Vec::new();
            if count_lines { parts.push(format!("{:>8}", lines)); }
            if count_words { parts.push(format!("{:>8}", words)); }
            if count_chars { parts.push(format!("{:>8}", chars)); }
            parts.push(file.clone());
            println!("{}", parts.join(" "));
        }
    }

    Ok(())
}

pub fn head(command: &Command) -> Result<()> {
    let mut n = 10;
    let mut file_path = None;

    let mut i = 0;
    while i < command.args.len() {
        if command.args[i] == "-n" && i + 1 < command.args.len() {
            n = command.args[i + 1].parse::<usize>().unwrap_or(10);
            i += 2;
        } else {
            file_path = Some(&command.args[i]);
            i += 1;
        }
    }

    let file = file_path.ok_or_else(|| anyhow::anyhow!("head: missing file"))?;

    let contents = fs::read_to_string(file)?;
    for (i, line) in contents.lines().enumerate() {
        if i >= n {
            break;
        }
        println!("{}", line);
    }

    Ok(())
}

pub fn tail(command: &Command) -> Result<()> {
    let mut n = 10;
    let mut file_path = None;

    let mut i = 0;
    while i < command.args.len() {
        if command.args[i] == "-n" && i + 1 < command.args.len() {
            n = command.args[i + 1].parse::<usize>().unwrap_or(10);
            i += 2;
        } else {
            file_path = Some(&command.args[i]);
            i += 1;
        }
    }

    let file = file_path.ok_or_else(|| anyhow::anyhow!("tail: missing file"))?;

    let contents = fs::read_to_string(file)?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = if lines.len() > n { lines.len() - n } else { 0 };

    for line in &lines[start..] {
        println!("{}", line);
    }

    Ok(())
}

pub fn sort_cmd(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("sort: missing file operand");
    }

    let reverse = command.args.contains(&"-r".to_string());

    for file in &command.args {
        if file.starts_with('-') {
            continue;
        }

        let path = Path::new(file.as_str());
        if !path.exists() {
            eprintln!("sort: {}: No such file or directory", file);
            continue;
        }

        let contents = fs::read_to_string(path)?;
        let mut lines: Vec<&str> = contents.lines().collect();
        
        if reverse {
            lines.sort_by(|a, b| b.cmp(a));
        } else {
            lines.sort();
        }

        for line in lines {
            println!("{}", line);
        }
    }

    Ok(())
}

pub fn uniq_cmd(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("uniq: missing file operand");
    }

    let count = command.args.contains(&"-c".to_string());

    for file in &command.args {
        if file.starts_with('-') {
            continue;
        }

        let path = Path::new(file.as_str());
        if !path.exists() {
            eprintln!("uniq: {}: No such file or directory", file);
            continue;
        }

        let contents = fs::read_to_string(path)?;
        let lines: Vec<&str> = contents.lines().collect();
        
        let mut prev = "";
        let mut line_count = 0;

        for line in lines {
            if line == prev {
                line_count += 1;
            } else {
                if !prev.is_empty() {
                    if count {
                        println!("{:>7} {}", line_count, prev);
                    } else {
                        println!("{}", prev);
                    }
                }
                prev = line;
                line_count = 1;
            }
        }

        if !prev.is_empty() {
            if count {
                println!("{:>7} {}", line_count, prev);
            } else {
                println!("{}", prev);
            }
        }
    }

    Ok(())
}

pub fn which_cmd(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("which: missing command");
    }

    for cmd in &command.args {
        match which::which(cmd) {
            Ok(path) => println!("{}", path.display()),
            Err(_) => eprintln!("{} not found", cmd),
        }
    }

    Ok(())
}