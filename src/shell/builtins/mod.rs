pub mod core;

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
                "cd".to_string(),
                "pwd".to_string(),
                "echo".to_string(),
                "export".to_string(),
                "help".to_string(),
                "clear".to_string(),
            ],
        }
    }

    pub fn is_builtin(&self, name: &str) -> bool {
        self.commands.contains(&name.to_string())
    }

    pub fn execute(&self, command: &Command, env: &mut Environment) -> Result<()> {
        match command.name.as_str() {
            "cd" => core::cd(command, env),
            "pwd" => core::pwd(env),
            "echo" => core::echo(command),
            "export" => core::export(command, env),
            "help" => core::help(),
            "clear" => core::clear(),
            _ => anyhow::bail!("Unknown built-in: {}", command.name),
        }
    }
}