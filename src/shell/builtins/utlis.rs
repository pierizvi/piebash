use anyhow::Result;
use colored::*;
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

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
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

    for file in &command.args {
        let path = Path::new(file);
        if !path.exists() {
            eprintln!("wc: {}: No such file or directory", file);
            continue;
        }

        let contents = fs::read_to_string(path)?;
        let lines = contents.lines().count();
        let words = contents.split_whitespace().count();
        let chars = contents.len();

        println!("{:>8} {:>8} {:>8} {}", lines, words, chars, file);
    }

    Ok(())
}

pub fn head(command: &Command) -> Result<()> {
    let n = if command.args.contains(&"-n".to_string()) {
        if let Some(idx) = command.args.iter().position(|a| a == "-n") {
            if idx + 1 < command.args.len() {
                command.args[idx + 1].parse::<usize>().unwrap_or(10)
            } else {
                10
            }
        } else {
            10
        }
    } else {
        10
    };

    let file = command.args.iter()
        .find(|a| !a.starts_with('-') && *a != &n.to_string())
        .ok_or_else(|| anyhow::anyhow!("head: missing file"))?;

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
    let n = if command.args.contains(&"-n".to_string()) {
        if let Some(idx) = command.args.iter().position(|a| a == "-n") {
            if idx + 1 < command.args.len() {
                command.args[idx + 1].parse::<usize>().unwrap_or(10)
            } else {
                10
            }
        } else {
            10
        }
    } else {
        10
    };

    let file = command.args.iter()
        .find(|a| !a.starts_with('-') && *a != &n.to_string())
        .ok_or_else(|| anyhow::anyhow!("tail: missing file"))?;

    let contents = fs::read_to_string(file)?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = if lines.len() > n { lines.len() - n } else { 0 };

    for line in &lines[start..] {
        println!("{}", line);
    }

    Ok(())
}