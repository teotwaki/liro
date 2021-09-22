use mobc_redis::redis;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("client initialization error: {0}")]
    Client(redis::RedisError),
    #[error("unable to get connection from pool: {0}")]
    Pool(#[from] mobc::Error<redis::RedisError>),
    #[error("error while running command: {0}")]
    Command(#[from] redis::RedisError),
}

pub type Result<T> = std::result::Result<T, Error>;
