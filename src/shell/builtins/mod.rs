pub mod core;
pub mod filesystem;
pub mod text;

use anyhow::Result;
use crate::shell::parser::Command;
use crate::shell::environment::Environment;

pub struct Builtins {
    commands: Vec<String>,
}

impl Builtins {
    pub fn new() -> Self {
        Self {
            commands: vec![
                // Navigation
                "cd".to_string(),
                "pwd".to_string(),
                
                // File operations
                "ls".to_string(),
                "cat".to_string(),
                "touch".to_string(),
                "mkdir".to_string(),
                "rm".to_string(),
                "cp".to_string(),
                "mv".to_string(),
                
                // Text processing
                "echo".to_string(),
                "grep".to_string(),
                
                // System
                "export".to_string(),
                "help".to_string(),
                "clear".to_string(),
                "env".to_string(),
            ],
        }
    }

    pub fn is_builtin(&self, name: &str) -> bool {
        self.commands.contains(&name.to_string())
    }

    pub fn execute(&self, command: &Command, env: &mut Environment) -> Result<()> {
        match command.name.as_str() {
            // Navigation
            "cd" => core::cd(command, env),
            "pwd" => core::pwd(env),
            
            // File operations
            "ls" => filesystem::ls(command, env),
            "cat" => filesystem::cat(command),
            "touch" => filesystem::touch(command),
            "mkdir" => filesystem::mkdir(command),
            "rm" => filesystem::rm(command),
            "cp" => filesystem::cp(command),
            "mv" => filesystem::mv(command),
            
            // Text processing
            "echo" => core::echo(command),
            "grep" => text::grep(command),
            
            // System
            "export" => core::export(command, env),
            "help" => core::help(),
            "clear" => core::clear(),
            "env" => core::env(env),
            
            _ => anyhow::bail!("Unknown built-in: {}", command.name),
        }
    }
}