use super::{Error, Result};
use crate::config;
use mobc_redis::{
    redis::{self, AsyncCommands, FromRedisValue},
    RedisConnectionManager,
};

pub type Pool = mobc::Pool<RedisConnectionManager>;
pub type Connection = mobc::Connection<RedisConnectionManager>;

pub async fn connect() -> Result<Pool> {
    trace!("connect() called");
    let redis_uri = config::redis_uri();
    let client = redis::Client::open(redis_uri).map_err(Error::Client)?;

    let manager = RedisConnectionManager::new(client);
    let pool = mobc::Pool::builder().max_open(20).build(manager);

    Ok(pool)
}

async fn get_connection(pool: &Pool) -> Result<Connection> {
    trace!("get_connection() called");
    Ok(pool.get().await?)
}

pub async fn set(pool: &Pool, key: &str, value: &str) -> Result<()> {
    trace!("set() called");
    let mut conn = get_connection(pool).await?;

    conn.set(key, value).await?;
    Ok(())
}

pub async fn set_ttl(pool: &Pool, key: &str, value: &str, ttl: usize) -> Result<()> {
    trace!("set_ttl() called");
    let mut conn = get_connection(pool).await?;

    conn.set_ex(key, value, ttl).await?;
    Ok(())
}

pub async fn get(pool: &Pool, key: &str) -> Result<Option<String>> {
    trace!("get() called");
    let mut conn = get_connection(pool).await?;

    let value = conn.get(key).await?;
    Ok(match value {
        redis::Value::Nil => None,
        _ => Some(FromRedisValue::from_redis_value(&value).map_err(Error::Type)?),
    })
}

pub async fn keys(pool: &Pool, prefix: &str) -> Result<Vec<String>> {
    trace!("keys() called");
    let mut conn = get_connection(pool).await?;

    let value = conn.keys(prefix).await?;

    Ok(FromRedisValue::from_redis_value(&value).map_err(Error::Type)?)
}
