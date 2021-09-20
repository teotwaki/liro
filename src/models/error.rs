use crate::{db, lichess};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("database error: {0}")]
    Database(#[from] db::Error),
    #[error("lichess error: {0}")]
    Lichess(#[from] lichess::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
