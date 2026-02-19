use anyhow::Result;
use colored::*;
use tokio::process::Command;
use std::process::Stdio;

use crate::shell::parser::Command as ShellCommand;
use crate::runtime::RuntimeManager;

pub async fn pip_install(command: &ShellCommand, runtime_manager: &RuntimeManager) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("pip: missing package name");
    }

    // Ensure Python runtime is installed
    let python_runtime = runtime_manager.ensure_runtime("python").await?;
    
    // Find pip executable
    let pip_path = python_runtime.path.join("Scripts").join("pip.exe");
    let pip_path = if pip_path.exists() {
        pip_path
    } else {
        python_runtime.path.join("bin").join("pip")
    };

    if !pip_path.exists() {
        anyhow::bail!("pip not found in Python installation");
    }

    println!("{} Installing Python packages...", "[PIP]".cyan().bold());

    // Run pip install
    let mut cmd = Command::new(&pip_path);
    cmd.arg("install");
    cmd.args(&command.args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("pip install failed");
    }

    println!("{} Installation complete!", "[OK]".green().bold());
    Ok(())
}

pub async fn npm_install(command: &ShellCommand, runtime_manager: &RuntimeManager) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("npm: missing package name");
    }

    let node_runtime = runtime_manager.ensure_runtime("node").await?;
    
    let npm_path = if cfg!(windows) {
        node_runtime.path.join("npm.cmd")
    } else {
        node_runtime.path.join("bin").join("npm")
    };

    if !npm_path.exists() {
        anyhow::bail!("npm not found in Node.js installation");
    }

    println!("{} Installing Node.js packages...", "[NPM]".cyan().bold());

    let mut cmd = Command::new(&npm_path);
    cmd.arg("install");
    cmd.arg("-g");
    cmd.args(&command.args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("npm install failed");
    }

    println!("{} Installation complete!", "[OK]".green().bold());
    Ok(())
}

pub async fn cargo_install(command: &ShellCommand, runtime_manager: &RuntimeManager) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("cargo: missing package name");
    }

    let rust_runtime = runtime_manager.ensure_runtime("rust").await?;
    
    let cargo_path = if cfg!(windows) {
        rust_runtime.path.join("bin").join("cargo.exe")
    } else {
        rust_runtime.path.join("bin").join("cargo")
    };

    if !cargo_path.exists() {
        anyhow::bail!("cargo not found in Rust installation");
    }

    println!("{} Installing Rust packages...", "[CARGO]".cyan().bold());

    let mut cmd = Command::new(&cargo_path);
    cmd.arg("install");
    cmd.args(&command.args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("cargo install failed");
    }

    println!("{} Installation complete!", "[OK]".green().bold());
    Ok(())
}

pub async fn gem_install(command: &ShellCommand, runtime_manager: &RuntimeManager) -> Result<()> {
    if command.args.is_empty() {
        anyhow::bail!("gem: missing package name");
    }

    let ruby_runtime = runtime_manager.ensure_runtime("ruby").await?;
    
    let gem_path = if cfg!(windows) {
        ruby_runtime.path.join("bin").join("gem.exe")
    } else {
        ruby_runtime.path.join("bin").join("gem")
    };

    if !gem_path.exists() {
        anyhow::bail!("gem not found in Ruby installation");
    }

    println!("{} Installing Ruby gems...", "[GEM]".cyan().bold());

    let mut cmd = Command::new(&gem_path);
    cmd.arg("install");
    cmd.args(&command.args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("gem install failed");
    }

    println!("{} Installation complete!", "[OK]".green().bold());
    Ok(())
}