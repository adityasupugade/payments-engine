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

impl From<csv_async::Error> for Error {
    fn from(_err: csv_async::Error) -> Error {
        Error::new(ErrorKind::Unknown("Unable to write to ObjectBMap channel".to_string()))
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Error {
        Error::new(ErrorKind::JoinError(err))
    }
}

pub enum ErrorKind {
    IO(std::io::Error),
    TokioSenderError(String),
    JoinError(tokio::task::JoinError),
    StoreError(String),
    EngineError(String),
    WrongClientError(u32, u16, u16),
    InsufficientAvailableFunds,
    DoubleDispute(u32),
    WrongTransactionRef(u32),
    Unknown(String),
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::IO(err) => write!(f, "{}", err),
            ErrorKind::TokioSenderError(msg) => {
                write!(f, "Failed to send on tokio channel. Error {}", msg)
            },
            ErrorKind::JoinError(err) => write!(f, "{}", err),
            ErrorKind::StoreError(msg) => write!(f, "Store error {}", msg),
            ErrorKind::EngineError(msg) => write!(f, "Engine error {}", msg),
            ErrorKind::WrongClientError(txn_id, client_id, wrong_id) => {
                write!(f, "Wrong client_id in transaction: {}, expected: {}, got: {}", txn_id, client_id, wrong_id)
            },
            ErrorKind::InsufficientAvailableFunds => write!(f, "Insufficient Available Funds"),
            ErrorKind::DoubleDispute(txn_id) => {
                write!(f, "Double dispute for transaction: {}", txn_id)
            },
            ErrorKind::WrongTransactionRef(txn_id) => {
                write!(f, "Wrong reference for transaction: {}", txn_id)
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
