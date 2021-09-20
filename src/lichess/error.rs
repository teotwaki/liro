use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("invalid authentication error: {0}")]
    InvalidAuthentication(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
