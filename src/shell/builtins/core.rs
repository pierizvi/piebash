use anyhow::Result;
use colored::*;

use crate::shell::parser::Command;
use crate::shell::environment::Environment;

pub fn cd(command: &Command, env: &mut Environment) -> Result<()> {
    let target = if command.args.is_empty() {
        env.get_home_dir()
    } else {
        let path = &command.args[0];
        if path == "~" {
            env.get_home_dir()
        } else if path.starts_with("~/") {
            env.get_home_dir().join(&path[2..])
        } else {
            env.get_cwd().join(path)
        }
    };

    if !target.exists() {
        anyhow::bail!("cd: no such file or directory: {}", target.display());
    }

    if !target.is_dir() {
        anyhow::bail!("cd: not a directory: {}", target.display());
    }

    env.set_cwd(target)?;
    Ok(())
}

pub fn pwd(env: &Environment) -> Result<()> {
    println!("{}", env.get_cwd().display());
    Ok(())
}

pub fn echo(command: &Command) -> Result<()> {
    println!("{}", command.args.join(" "));
    Ok(())
}

pub fn export(command: &Command, env: &mut Environment) -> Result<()> {
    if command.args.is_empty() {
        // Print all environment variables
        for (key, value) in env.get_all_vars() {
            println!("{}={}", key, value);
        }
    } else {
        // Set environment variable
        for arg in &command.args {
            if let Some(pos) = arg.find('=') {
                let key = &arg[..pos];
                let value = &arg[pos + 1..];
                env.set_var(key, value);
            } else {
                anyhow::bail!("export: invalid syntax: {}", arg);
            }
        }
    }
    Ok(())
}

pub fn env(environment: &Environment) -> Result<()> {
    let mut vars: Vec<_> = environment.get_all_vars().iter().collect();
    vars.sort_by_key(|(k, _)| *k);
    
    for (key, value) in vars {
        println!("{}={}", key, value);
    }
    Ok(())
}

pub fn help() -> Result<()> {
    println!("{}", "╔═══════════════════════════════════════════════╗".cyan());
    println!("{}", "║          PieBash - Command Reference          ║".cyan().bold());
    println!("{}", "╚═══════════════════════════════════════════════╝".cyan());
    println!();
    
    println!("{}", " File & Directory Commands:".yellow().bold());
    println!("  {}    - List directory contents", "ls [-la]".cyan());
    println!("  {}     - Change directory", "cd <dir>".cyan());
    println!("  {}      - Print working directory", "pwd".cyan());
    println!("  {}    - Display file contents", "cat <file>".cyan());
    println!("  {}  - Create empty file", "touch <file>".cyan());
    println!("  {} - Create directory", "mkdir <dir>".cyan());
    println!("  {}   - Remove file/directory", "rm [-rf] <file>".cyan());
    println!("  {}   - Copy file", "cp <src> <dst>".cyan());
    println!("  {}   - Move/rename file", "mv <src> <dst>".cyan());
    println!();
    
    println!("{}", " Text Processing:".yellow().bold());
    println!("  {}   - Print text", "echo <text>".cyan());
    println!("  {} - Search in files", "grep <pattern> <file>".cyan());
    println!();
    
    println!("{}", " System Commands:".yellow().bold());
    println!("  {} - Set environment variable", "export VAR=value".cyan());
    println!("  {}     - Show all variables", "env".cyan());
    println!("  {}   - Clear screen", "clear".cyan());
    println!("  {}    - Show this help", "help".cyan());
    println!("  {}    - Exit shell", "exit".cyan());
    println!();
    
    println!("{}", " Code Execution:".yellow().bold());
    println!("  {} - Execute Python script", "python script.py".cyan());
    println!("  {}   - Execute Node.js script", "node app.js".cyan());
    println!("  {} - Execute Java file", "java Main.java".cyan());
    println!("  {} - Execute inline code", "@python print('hi')".cyan());
    println!();
    
    println!("{}", " Tips:".yellow());
    println!("  • Language runtimes auto-download when needed");
    println!("  • Use {} for home directory", "~".cyan());
    println!("  • Tab completion coming soon!");
    println!();
    
    Ok(())
}

pub fn clear() -> Result<()> {
    print!("\x1B[2J\x1B[1;1H");
    Ok(())
}