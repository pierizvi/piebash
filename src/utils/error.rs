use thiserror::Error;

#[derive(Error, Debug)]
pub enum PieBashError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}