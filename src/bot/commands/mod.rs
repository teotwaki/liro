mod account;
mod meta;
mod rating_update;

use super::Data;
use crate::models;
use poise::serenity_prelude::{Error as SerenityError, GuildId};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("model error: {0}")]
    Model(#[from] models::Error),
    #[error("Discord error: {0}")]
    Discord(#[source] Box<SerenityError>),
    #[error("this command can only be used in a server")]
    GuildOnly,
}

impl From<SerenityError> for Error {
    fn from(error: SerenityError) -> Self {
        Self::Discord(Box::new(error))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
type Context<'a> = poise::Context<'a, Data, Error>;

fn guild_id(ctx: Context<'_>) -> Result<GuildId> {
    ctx.guild_id().ok_or(Error::GuildOnly)
}

pub fn registry() -> Vec<poise::Command<Data, Error>> {
    vec![
        meta::help(),
        account::account(),
        account::gdpr(),
        rating_update::rating(),
        account::link(),
        account::unlink(),
    ]
}
