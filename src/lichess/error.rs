use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("invalid authentication error: {0}")]
    InvalidAuthenticationError(#[from] serde_json::Error),
    #[error("received unexpected status from lichess.org")]
    UnexpectedStatusError,
}

pub type Result<T> = std::result::Result<T, Error>;
