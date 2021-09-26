use super::{Error, Result};
use crate::config;
use mobc_redis::{
    redis::{self, AsyncCommands},
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

pub async fn set<K, V>(pool: &Pool, key: K, value: V) -> Result<()>
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    trace!("set() called");
    let mut conn = get_connection(pool).await?;

    conn.set(key.as_ref(), value.as_ref()).await?;
    Ok(())
}

pub async fn set_ttl<K, V>(pool: &Pool, key: K, value: V, ttl: usize) -> Result<()>
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    trace!("set_ttl() called");
    let mut conn = get_connection(pool).await?;

    conn.set_ex(key.as_ref(), value.as_ref(), ttl).await?;
    Ok(())
}

pub async fn get<K>(pool: &Pool, key: K) -> Result<Option<String>>
where
    K: AsRef<str>,
{
    trace!("get() called");
    let mut conn = get_connection(pool).await?;

    Ok(conn.get(key.as_ref()).await?)
}

pub async fn mget<V>(pool: &Pool, keys: V) -> Result<Vec<String>>
where
    V: Into<Vec<String>>,
{
    trace!("mget() called");
    let mut conn = get_connection(pool).await?;
    let keys = keys.into();

    if keys.len() == 1 {
        Ok(vec![conn.get(keys).await?])
    } else {
        Ok(conn.get(keys).await?)
    }
}

pub async fn keys<K>(pool: &Pool, prefix: K) -> Result<Vec<String>>
where
    K: AsRef<str>,
{
    trace!("keys() called");
    let mut conn = get_connection(pool).await?;

    Ok(conn.keys(prefix.as_ref()).await?)
}

pub async fn del<K>(pool: &Pool, key: K) -> Result<bool>
where
    K: AsRef<str>,
{
    trace!("del() called");
    let mut conn = get_connection(pool).await?;

    Ok(conn.del(key.as_ref()).await?)
}
