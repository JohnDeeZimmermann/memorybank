use std::fmt;

#[derive(Debug)]
pub enum CliError {
    NotInitialized(String),
    NotFound(String),
    Validation(String),
    Storage(String),
    Database(String),
    Replay(String),
}

impl CliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotInitialized(_) => "NOT_INITIALIZED",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Validation(_) => "VALIDATION",
            Self::Storage(_) => "STORAGE",
            Self::Database(_) => "DATABASE",
            Self::Replay(_) => "REPLAY",
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::NotInitialized(message)
            | Self::NotFound(message)
            | Self::Validation(message)
            | Self::Storage(message)
            | Self::Database(message)
            | Self::Replay(message) => message,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERROR: {} {}", self.code(), self.message())
    }
}

impl std::error::Error for CliError {}

pub type CliResult<T> = Result<T, CliError>;
