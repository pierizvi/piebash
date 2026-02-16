use anyhow::Result;
use colored::*;
use std::fs;
use std::path::Path;
use regex::Regex;

use crate::shell::parser::Command;

pub fn grep(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("grep: missing pattern");
    }

    let pattern = &command.args[0];
    let regex = Regex::new(pattern)?;

    if command.args.len() == 1 {
        anyhow::bail!("grep: missing file operand");
    }

    for file in &command.args[1..] {
        let path = Path::new(file.as_str());  
        
        if !path.exists() {
            eprintln!("grep: {}: No such file or directory", file);
            continue;
        }

        let contents = fs::read_to_string(path)?;
        
        for (line_num, line) in contents.lines().enumerate() {
            if regex.is_match(line) {
                println!("{}:{}:{}", file.cyan(), (line_num + 1).to_string().green(), line);
            }
        }
    }

    Ok(())
}