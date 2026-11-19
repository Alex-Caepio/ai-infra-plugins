//! Error and result types for standalone agent-guardrails.
use std::fmt;

/// Broad classification of guardrail failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    InvalidInput,
    Policy,
    Conflict,
    Unavailable,
    Io,
}

/// Owned runtime error with optional machine-readable diagnostic code.
#[derive(Clone, Debug)]
pub struct GuardError {
    pub kind: ErrorKind,
    pub code: Option<String>,
    pub message: String,
}

impl GuardError {
    /// Construct an unclassified error for a specific kind.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: None,
            message: message.into(),
        }
    }

    /// Construct an error carrying a stable machine code.
    pub fn coded(kind: ErrorKind, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: Some(code.into()),
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Policy, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unavailable, message)
    }
}

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GuardError {}

impl From<std::io::Error> for GuardError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, error.to_string())
    }
}

impl From<serde_json::Error> for GuardError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(ErrorKind::InvalidInput, error.to_string())
    }
}

impl From<regex::Error> for GuardError {
    fn from(error: regex::Error) -> Self {
        Self::new(ErrorKind::InvalidInput, error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, GuardError>;
