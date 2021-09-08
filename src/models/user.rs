use crate::db::{self, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    discord_id: u64,
    lichess_username: String,
    rating: Option<i16>,
}

impl User {
    fn key(id: u64) -> String {
        trace!("User::key() called");
        format!("users:{}", id)
    }

    pub async fn new(pool: &db::Pool, discord_id: u64, lichess_username: String) -> Result<User> {
        trace!("User::new() called");
        let user = User {
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
        db::set(pool, &User::key(self.discord_id), &serialized).await?;

        Ok(())
    }

    pub async fn find(pool: &db::Pool, id: u64) -> Result<Option<User>> {
        debug!("Looking up discord_id={}", id);
        let serialized = db::get(pool, &User::key(id)).await?;
        let user = serde_json::from_str(&serialized)?;
        debug!("Found {}", &user);

        Ok(Some(user))
    }

    pub fn lichess_username(&self) -> &str {
        trace!("User::lichess_username() called");
        &self.lichess_username
    }

    pub async fn update_rating(&mut self, pool: &db::Pool, rating: i16) -> Result<()> {
        trace!("User::update_rating() called");
        debug!("Updating {}", self);
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
