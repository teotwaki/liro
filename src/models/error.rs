use crate::db;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("database error: {0}")]
    DatabaseError(#[from] db::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
