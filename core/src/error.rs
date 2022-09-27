use std::sync::Arc;

use crate::transactions::Transaction;

#[derive(Clone, Debug)]
pub struct Error {
    pub kind: Arc<ErrorKind>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Error {
            kind: Arc::new(kind),
        }
    }
}

impl From<std::string::String> for Error {
    fn from(err: std::string::String) -> Error {
        Error::new(ErrorKind::Unknown(err))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::new(ErrorKind::IO(err))
    }
}

impl From<tokio::sync::mpsc::error::SendError<Transaction>> for Error {
    fn from(_err: tokio::sync::mpsc::error::SendError<Transaction>) -> Error {
        Error::new(ErrorKind::TokioSenderError("Unable to write to ObjectBMap channel".to_string()))
    }
}

pub enum ErrorKind {
    IO(std::io::Error),
    TokioSenderError(String),
    Unknown(String),
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::IO(err) => write!(f, "{}", err),
            ErrorKind::TokioSenderError(msg) => {
                write!(f, "Failed to send on tokio channel. Error {}", msg)
            },
            ErrorKind::Unknown(msg) => write!(f, "Unknown error {}", msg),
        }
    }
}

impl std::fmt::Debug for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}", self))
    }
}
