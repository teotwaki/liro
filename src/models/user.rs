use super::Result;
use crate::db;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    guild_id: u64,
    discord_id: u64,
    lichess_username: String,
    rating: Option<i16>,
}

fn key(guild_id: u64, discord_id: u64) -> String {
    trace!("key() called");
    format!("guilds:{}:users:{}", guild_id, discord_id)
}

impl User {
    fn key(&self) -> String {
        trace!("User::key() called");
        key(self.guild_id, self.discord_id)
    }

    pub async fn new(
        pool: &db::Pool,
        guild_id: u64,
        discord_id: u64,
        lichess_username: String,
    ) -> Result<User> {
        trace!("User::new() called");
        let user = User {
            guild_id,
            discord_id,
            lichess_username,
            rating: None,
        };

        user.save(pool).await?;

        Ok(user)
    }

    async fn save(&self, pool: &db::Pool) -> Result<()> {
        trace!("User::save() called");
        debug!("Saving {}", &self);
        let serialized = serde_json::to_string(self)?;
        db::set(pool, &self.key(), &serialized).await?;

        Ok(())
    }

    pub async fn find(pool: &db::Pool, guild_id: u64, discord_id: u64) -> Result<Option<User>> {
        trace!("User::find() called");
        debug!("Looking up user with discord_id={}", discord_id);

        match db::get(pool, &key(guild_id, discord_id)).await? {
            Some(serialized) => {
                let user = serde_json::from_str(&serialized)?;
                debug!("Found {}", user);
                Ok(Some(user))
            }
            None => {
                debug!("User not found");
                Ok(None)
            }
        }
    }

    pub fn lichess_username(&self) -> &str {
        trace!("User::lichess_username() called");
        &self.lichess_username
    }

    pub async fn update_rating(&mut self, pool: &db::Pool, rating: i16) -> Result<()> {
        trace!("User::update_rating() called");
        debug!(
            "Updating rating discord_id={} old_rating={:?} new_rating={}",
            self.discord_id, self.rating, rating
        );

        self.rating = Some(rating);
        self.save(pool).await?;

        Ok(())
    }

    pub fn rating(&self) -> Option<i16> {
        trace!("User::rating() called");
        self.rating
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        trace!("User::fmt() called");
        match self.rating {
            Some(rating) => write!(
                f,
                "User<discord_id={}, lichess_username={} rating={}>",
                self.discord_id, self.lichess_username, rating
            ),
            None => write!(
                f,
                "User<discord_id={}, lichess_username={} rating=None>",
                self.discord_id, self.lichess_username
            ),
        }
    }
}
