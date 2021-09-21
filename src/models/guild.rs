use super::Result;
use crate::db;

pub struct Guild;

impl Guild {
    pub async fn count(pool: &db::Pool) -> Result<usize> {
        trace!("Guild::count() called");

        let keys = db::keys(pool, "guilds:*").await?;

        let mut guild_ids: Vec<&str> = keys
            .iter()
            .map(|k| k.split(':').collect::<Vec<&str>>()[1])
            .collect();

        guild_ids.sort_unstable();
        guild_ids.dedup();

        Ok(guild_ids.len())
    }
}
