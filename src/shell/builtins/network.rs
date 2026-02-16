use anyhow::Result;
use colored::*;
use std::fs::File;
use std::io::Write;

use crate::shell::parser::Command;

pub async fn wget(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("wget: missing URL");
    }

    let url = &command.args[0];
    
    let filename = if let Some(idx) = command.args.iter().position(|a| a == "-O") {
        if idx + 1 < command.args.len() {
            command.args[idx + 1].clone()
        } else {
            anyhow::bail!("wget: -O requires filename");
        }
    } else {
        url.split('/').last().unwrap_or("index.html").to_string()
    };

    println!("{} Downloading from {}...", "[WGET]".cyan(), url.cyan());

    let response = reqwest::get(url).await?;
    let total_size = response.content_length().unwrap_or(0);
    
    let bytes = response.bytes().await?;
    
    let mut file = File::create(&filename)?;
    file.write_all(&bytes)?;

    println!("{} Saved to {}", "[OK]".green(), filename.green().bold());
    println!("  Size: {} bytes", total_size);

    Ok(())
}

pub async fn curl(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("curl: missing URL");
    }

    let url = &command.args[0];
    let save_output = command.args.contains(&"-o".to_string()) || command.args.contains(&"-O".to_string());
    
    let response = reqwest::get(url).await?;
    
    if save_output {
        let filename = if let Some(idx) = command.args.iter().position(|a| a == "-o") {
            if idx + 1 < command.args.len() {
                command.args[idx + 1].clone()
            } else {
                url.split('/').last().unwrap_or("output").to_string()
            }
        } else {
            url.split('/').last().unwrap_or("output").to_string()
        };
        
        let bytes = response.bytes().await?;
        let mut file = File::create(&filename)?;
        file.write_all(&bytes)?;
        println!("Saved to {}", filename);
    } else {
        let text = response.text().await?;
        print!("{}", text);
    }

    Ok(())
}