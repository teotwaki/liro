use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
