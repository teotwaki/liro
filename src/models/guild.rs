use super::Result;
use crate::db;

pub struct Guild;

impl Guild {
    pub async fn count(pool: &db::Pool) -> Result<usize> {
        trace!("Guild::count() called");

        let keys = db::keys(pool, "guilds:*").await?;

        Ok(keys.len())
    }
}
