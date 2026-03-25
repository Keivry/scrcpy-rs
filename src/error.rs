// SPDX-License-Identifier: MIT OR Apache-2.0

use {
    std::{fmt, io},
    thiserror::Error,
};

#[derive(Debug)]
pub struct IoError {
    source: Option<Box<io::Error>>,
    message: String,
}

impl IoError {
    pub fn new(source: io::Error) -> Self {
        let message = source.to_string();
        Self {
            source: Some(Box::new(source)),
            message,
        }
    }

    pub fn new_with_message(message: impl Into<String>) -> Self {
        Self {
            source: None,
            message: message.into(),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn message(&self) -> &str { &self.message }

    pub fn kind(&self) -> io::ErrorKind {
        self.source
            .as_ref()
            .map(|e| e.kind())
            .unwrap_or(io::ErrorKind::Other)
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.kind())
    }
}

impl std::error::Error for IoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

#[derive(Debug, Error)]
pub enum ScrcpyError {
    #[error("I/O error: {0}")]
    IoError(IoError),

    #[error("Scrcpy protocol error: {0}")]
    ProtocolError(String),

    #[error("Unexpected error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ScrcpyError>;

impl ScrcpyError {
    pub fn is_timeout(&self) -> bool {
        if let Self::IoError(err) = self {
            matches!(
                err.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            )
        } else {
            false
        }
    }
}

impl From<io::Error> for ScrcpyError {
    fn from(source: io::Error) -> Self { Self::IoError(IoError::new(source)) }
}
