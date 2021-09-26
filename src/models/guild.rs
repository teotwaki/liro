use super::Result;
use crate::db;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct Guild {
    id: u64,
    name: String,
}

fn key(guild_id: u64) -> String {
    trace!("key() called");
    format!("guilds:{}", guild_id)
}

impl Guild {
    fn key(&self) -> String {
        trace!("Guild::key() called");
        key(self.id)
    }

    pub async fn new<N>(pool: &db::Pool, id: u64, name: N) -> Result<Self>
    where
        N: Into<String>,
    {
        trace!("Guild::new() called");
        let guild = Guild {
            id,
            name: name.into(),
        };

        guild.save(pool).await?;

        Ok(guild)
    }

    async fn save(&self, pool: &db::Pool) -> Result<()> {
        trace!("Guild::save() called");
        let serialized = serde_json::to_string(self)?;
        db::set(pool, self.key(), &serialized).await?;

        Ok(())
    }

    pub async fn count(pool: &db::Pool) -> Result<usize> {
        trace!("Guild::count() called");

        let keys = db::keys(pool, "guilds:*").await?;

        Ok(keys.len())
    }

    pub async fn delete(&self, pool: &db::Pool) -> Result<()> {
        trace!("Guild::delete() called");
        db::del(pool, self.key()).await?;

        Ok(())
    }

    pub async fn find(pool: &db::Pool, id: u64) -> Result<Option<Guild>> {
        trace!("Guild::find() called");
        match db::get(pool, key(id)).await? {
            Some(serialized) => {
                let guild = serde_json::from_str(&serialized)?;
                Ok(Some(guild))
            }
            None => Ok(None),
        }
    }
}

impl fmt::Display for Guild {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        trace!("Guild::fmt() called");
        write!(f, "Guild<id={} name={}>", self.id, self.name)
    }
}
