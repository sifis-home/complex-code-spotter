use thiserror::Error;

use crate::concurrent::ConcurrentErrors;

/// Error types.
#[derive(Debug, Error)]
pub enum Error {
    /// Wrong file content.
    #[error("Wrong content")]
    WrongContent,
    /// Impossible to guess a programming language.
    #[error("Impossible to guess the programming language")]
    UnknownLanguage,
    /// Impossible to retrieve function spaces.
    #[error("Impossible to retrieve function spaces")]
    NoSpaces,
    /// A general utf-8 conversion error.
    #[error("Utf-8 error")]
    Utf8(#[from] std::str::Utf8Error),
    /// Impossible to complete a non-utf8 conversion.
    #[error("Impossible to complete a non-utf8 conversion")]
    NonUtf8Conversion,
    /// Path format.
    #[error("{0}")]
    FormatPath(String),
    /// Concurrent failures.
    #[error("Concurrent failure: {0}")]
    Concurrent(String),
    /// Mutability access failures.
    #[error("Mutability failure: {0}")]
    Mutability(String),
    /// Less thresholds than complexity metrics.
    #[error("Each complexity metric MUST have a threshold.")]
    Thresholds,
    /// A more generic I/O error.
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("Json error")]
    /// A Json output error.
    JsonOutput(#[from] serde_json::Error),
}

impl From<crate::concurrent::ConcurrentErrors> for Error {
    fn from(e: ConcurrentErrors) -> Self {
        let value = match e {
            ConcurrentErrors::Producer(e) => format!("Producer: {e}"),
            ConcurrentErrors::Sender(e) => format!("Sender: {e}"),
            ConcurrentErrors::Receiver(e) => format!("Receiver: {e}"),
            ConcurrentErrors::Thread(e) => format!("Thread: {e}"),
        };
        Self::Concurrent(value)
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(e: std::sync::PoisonError<T>) -> Self {
        Self::Mutability(e.to_string())
    }
}

/// A specialized `Result` type.
pub type Result<T> = ::std::result::Result<T, Error>;
