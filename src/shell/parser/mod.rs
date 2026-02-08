pub mod command;
pub mod lexer;

pub use command::{Command, Redirect};
use anyhow::Result;
use self::lexer::Lexer;
use std::collections::HashMap;

pub struct CommandParser {
    lexer: Lexer,
}

impl CommandParser {
    pub fn new() -> Self {
        Self {
            lexer: Lexer::new(),
        }
    }

    pub fn parse(&self, input: &str) -> Result<Command> {
        self.parse_with_env(input, &HashMap::new())
    }

    pub fn parse_with_env(&self, input: &str, env: &HashMap<String, String>) -> Result<Command> {
        // Check for pipes
        if input.contains('|') {
            return self.parse_pipeline_with_env(input, env);
        }

        // Check for redirects
        if input.contains('>') {
            return self.parse_with_redirect_env(input, env);
        }

        // Simple command
        self.parse_simple_with_env(input, env)
    }

    fn parse_simple_with_env(&self, input: &str, env: &HashMap<String, String>) -> Result<Command> {
        let tokens = self.lexer.tokenize_with_env(input, env)?;

        if tokens.is_empty() {
            anyhow::bail!("Empty command");
        }

        let name = tokens[0].clone();
        let args = tokens[1..].to_vec();

        Ok(Command::new(name, args))
    }

    fn parse_with_redirect_env(&self, input: &str, env: &HashMap<String, String>) -> Result<Command> {
        let append = input.contains(">>");
        let redirect_op = if append { ">>" } else { ">" };
        
        let parts: Vec<&str> = input.splitn(2, redirect_op).collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid redirect syntax");
        }

        let cmd_part = parts[0].trim();
        let file_part = parts[1].trim();

        let mut command = self.parse_simple_with_env(cmd_part, env)?;
        command.redirect_stdout = Some(Redirect {
            target: file_part.to_string(),
            append,
        });

        Ok(command)
    }

    fn parse_pipeline_with_env(&self, input: &str, env: &HashMap<String, String>) -> Result<Command> {
        let parts: Vec<&str> = input.split('|').map(|s| s.trim()).collect();
        
        if parts.len() < 2 {
            anyhow::bail!("Invalid pipe syntax");
        }

        let mut commands: Vec<Command> = Vec::new();
        for part in parts {
            let cmd = self.parse_simple_with_env(part, env)?;
            commands.push(cmd);
        }

        let mut final_cmd = commands.pop().unwrap();
        while let Some(mut prev) = commands.pop() {
            prev.pipe_to = Some(Box::new(final_cmd));
            final_cmd = prev;
        }

        Ok(final_cmd)
    }
}