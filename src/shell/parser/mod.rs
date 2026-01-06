pub mod command;
pub mod lexer;

pub use command::Command;
use anyhow::Result;
use self::lexer::Lexer;

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
        // Tokenize input
        let tokens = self.lexer.tokenize(input)?;

        if tokens.is_empty() {
            anyhow::bail!("Empty command");
        }

        // First token is the command name
        let name = tokens[0].clone();

        // Rest are arguments
        let args = tokens[1..].to_vec();

        Ok(Command { name, args })
    }
}