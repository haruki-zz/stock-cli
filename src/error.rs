use std::sync::mpsc::RecvError;

use thiserror::Error;

pub use anyhow::Context;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Calamine(#[from] calamine::Error),
    #[error(transparent)]
    Chrono(#[from] chrono::ParseError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Recv(#[from] RecvError),
    #[error("operation cancelled by user")]
    Cancelled,
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    pub fn message<T: Into<String>>(msg: T) -> Self {
        AppError::Message(msg.into())
    }
}
