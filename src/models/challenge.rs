use crate::{
    db::{self, Result},
    lichess::auth,
};
use serde::{Deserialize, Serialize};
use std::fmt;

const TTL: usize = 86400;

#[derive(Serialize, Deserialize, Debug)]
pub struct Challenge {
    id: u64,
    discord_id: u64,
    code_verifier: Vec<u8>,
}

impl Challenge {
    fn key(id: u64) -> String {
        trace!("Challenge::key() called");
        format!("challenges:{}", id)
    }

    pub async fn new(pool: &db::Pool, discord_id: u64) -> Result<Challenge> {
        trace!("Challenge::new() called");
        let challenge = Self {
            id: rand::random(),
            discord_id,
            code_verifier: pkce::code_verifier(128),
        };

        challenge.save(pool).await?;

        Ok(challenge)
    }

    async fn save(&self, pool: &db::Pool) -> Result<()> {
        trace!("Challenge::save() called");
        let serialized = serde_json::to_string(self)?;
        db::set_ttl(pool, &Challenge::key(self.id), &serialized, TTL).await?;

        Ok(())
    }

    pub async fn find(pool: &db::Pool, id: u64) -> Result<Option<Challenge>> {
        trace!("Challenge::find() called");
        match db::get(pool, &Challenge::key(id)).await? {
            Some(serialized) => Ok(Some(serde_json::from_str(&serialized)?)),
            None => Ok(None),
        }
    }

    fn code_challenge(&self) -> String {
        trace!("Challenge::code_challenge() called");
        pkce::code_challenge(&self.code_verifier)
    }

    fn state(&self) -> String {
        trace!("Challenge::state() called");
        format!("{}", self.id)
    }

    pub fn discord_id(&self) -> u64 {
        trace!("Challenge::discord_id() called");
        self.discord_id
    }

    pub fn lichess_url(&self) -> String {
        trace!("Challenge::lichess_url() called");
        auth::oauth_url(&self.code_challenge(), &self.state())
    }

    pub fn code_verifier(&self) -> String {
        trace!("Challenge::code_verifier() called");
        match std::str::from_utf8(&self.code_verifier) {
            Ok(v) => v.to_string(),
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        }
    }
}

impl fmt::Display for Challenge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        trace!("Challenge::fmt() called");
        write!(f, "Challenge<id={} user_id={}>", self.id, self.discord_id)
    }
}
