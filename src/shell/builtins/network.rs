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
    
    // Determine output filename
    let filename = if let Some(idx) = command.args.iter().position(|a| a == "-O") {
        if idx + 1 < command.args.len() {
            command.args[idx + 1].clone()
        } else {
            anyhow::bail!("wget: -O requires filename");
        }
    } else {
        url.split('/').last().unwrap_or("index.html").to_string()
    };

    println!("Downloading {} ...", url.cyan());

    let response = reqwest::get(url).await?;
    let total_size = response.content_length().unwrap_or(0);
    
    let bytes = response.bytes().await?;
    
    let mut file = File::create(&filename)?;
    file.write_all(&bytes)?;

    println!("✓ Saved to {}", filename.green());
    println!("  Size: {} bytes", total_size);

    Ok(())
}

pub async fn curl(command: &Command) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("curl: missing URL");
    }

    let url = &command.args[0];
    
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    
    print!("{}", text);

    Ok(())
}