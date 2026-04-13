pub mod command;
pub mod lexer;

use self::lexer::Lexer;
use anyhow::Result;
pub use command::{ChainOperator, Command, Redirect};
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
        let input = input.trim();
        if input.is_empty() {
            anyhow::bail!("Empty command");
        }

        // Check for command chaining (&&, ||, ;)
        if let Some(cmd) = self.try_parse_chain(input, env)? {
            return Ok(cmd);
        }

        // Check for pipes
        if Self::has_top_level_pipe(input) {
            return self.parse_pipeline_with_env(input, env);
        }

        // Check for redirects
        if Self::find_top_level_redirect(input).is_some() {
            return self.parse_with_redirect_env(input, env);
        }

        // Simple command
        self.parse_simple_with_env(input, env)
    }

    fn try_parse_chain(
        &self,
        input: &str,
        env: &HashMap<String, String>,
    ) -> Result<Option<Command>> {
        if let Some(index) = Self::find_top_level_semicolon(input) {
            let first = self.parse_with_env(&input[..index], env)?;
            let second = self.parse_with_env(&input[index + 1..], env)?;
            return Ok(Some(first.with_chain(ChainOperator::Semicolon, second)));
        }

        if let Some((index, operator)) = Self::find_top_level_logical(input) {
            let first = self.parse_with_env(&input[..index], env)?;
            let second = self.parse_with_env(&input[index + 2..], env)?;
            return Ok(Some(first.with_chain(operator, second)));
        }

        Ok(None)
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

    fn parse_with_redirect_env(
        &self,
        input: &str,
        env: &HashMap<String, String>,
    ) -> Result<Command> {
        let (index, append) =
            Self::find_top_level_redirect(input).ok_or_else(|| anyhow::anyhow!("Invalid redirect syntax"))?;
        let redirect_len = if append { 2 } else { 1 };
        let cmd_part = input[..index].trim();
        let file_part = input[index + redirect_len..].trim();

        if cmd_part.is_empty() || file_part.is_empty() {
            anyhow::bail!("Invalid redirect syntax");
        }

        let mut command = self.parse_simple_with_env(cmd_part, env)?;
        command.redirect_stdout = Some(Redirect {
            target: file_part.to_string(),
            append,
        });

        Ok(command)
    }

    fn parse_pipeline_with_env(
        &self,
        input: &str,
        env: &HashMap<String, String>,
    ) -> Result<Command> {
        let parts = Self::split_top_level_pipes(input);

        if parts.len() < 2 {
            anyhow::bail!("Invalid pipe syntax");
        }

        let mut commands: Vec<Command> = Vec::new();
        for part in parts {
            let cmd = self.parse_pipeline_segment_with_env(part, env)?;
            commands.push(cmd);
        }

        let mut final_cmd = commands.pop().unwrap();
        while let Some(mut prev) = commands.pop() {
            prev.pipe_to = Some(Box::new(final_cmd));
            final_cmd = prev;
        }

        Ok(final_cmd)
    }

    fn parse_pipeline_segment_with_env(
        &self,
        input: &str,
        env: &HashMap<String, String>,
    ) -> Result<Command> {
        if Self::find_top_level_redirect(input).is_some() {
            self.parse_with_redirect_env(input, env)
        } else {
            self.parse_simple_with_env(input, env)
        }
    }

    fn find_top_level_semicolon(input: &str) -> Option<usize> {
        let bytes = input.as_bytes();
        let mut quote = None;
        let mut i = 0;

        while i < bytes.len() {
            match quote {
                Some(active) => {
                    if bytes[i] == active {
                        quote = None;
                    }
                }
                None => match bytes[i] {
                    b'\'' | b'"' => quote = Some(bytes[i]),
                    b';' => return Some(i),
                    _ => {}
                },
            }
            i += 1;
        }

        None
    }

    fn find_top_level_logical(input: &str) -> Option<(usize, ChainOperator)> {
        let bytes = input.as_bytes();
        let mut quote = None;
        let mut i = 0;

        while i + 1 < bytes.len() {
            match quote {
                Some(active) => {
                    if bytes[i] == active {
                        quote = None;
                    }
                    i += 1;
                }
                None => {
                    match bytes[i] {
                        b'\'' | b'"' => {
                            quote = Some(bytes[i]);
                            i += 1;
                        }
                        b'&' if bytes[i + 1] == b'&' => return Some((i, ChainOperator::And)),
                        b'|' if bytes[i + 1] == b'|' => return Some((i, ChainOperator::Or)),
                        _ => i += 1,
                    }
                }
            }
        }

        None
    }

    fn has_top_level_pipe(input: &str) -> bool {
        let bytes = input.as_bytes();
        let mut quote = None;
        let mut i = 0;

        while i < bytes.len() {
            match quote {
                Some(active) => {
                    if bytes[i] == active {
                        quote = None;
                    }
                }
                None => match bytes[i] {
                    b'\'' | b'"' => quote = Some(bytes[i]),
                    b'|' if i + 1 >= bytes.len() || bytes[i + 1] != b'|' => return true,
                    _ => {}
                },
            }
            i += 1;
        }

        false
    }

    fn find_top_level_redirect(input: &str) -> Option<(usize, bool)> {
        let bytes = input.as_bytes();
        let mut quote = None;
        let mut i = 0;

        while i < bytes.len() {
            match quote {
                Some(active) => {
                    if bytes[i] == active {
                        quote = None;
                    }
                    i += 1;
                }
                None => match bytes[i] {
                    b'\'' | b'"' => {
                        quote = Some(bytes[i]);
                        i += 1;
                    }
                    b'>' => {
                        let append = i + 1 < bytes.len() && bytes[i + 1] == b'>';
                        return Some((i, append));
                    }
                    _ => i += 1,
                },
            }
        }

        None
    }

    fn split_top_level_pipes<'a>(input: &'a str) -> Vec<&'a str> {
        let bytes = input.as_bytes();
        let mut quote = None;
        let mut start = 0;
        let mut i = 0;
        let mut parts = Vec::new();

        while i < bytes.len() {
            match quote {
                Some(active) => {
                    if bytes[i] == active {
                        quote = None;
                    }
                    i += 1;
                }
                None => match bytes[i] {
                    b'\'' | b'"' => {
                        quote = Some(bytes[i]);
                        i += 1;
                    }
                    b'|' if i + 1 >= bytes.len() || bytes[i + 1] != b'|' => {
                        parts.push(input[start..i].trim());
                        start = i + 1;
                        i += 1;
                    }
                    _ => i += 1,
                },
            }
        }

        parts.push(input[start..].trim());
        parts
    }
}
