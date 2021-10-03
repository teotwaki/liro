pub mod account;
pub mod meta;
pub mod rating_update;

use crate::models;
use serenity::{builder::CreateEmbed, prelude::SerenityError};
use thiserror::Error;

pub enum Response {
    Embed(CreateEmbed),
    PrivateEmbed(CreateEmbed),
    Sentence(String),
    PrivateSentence(String),
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("model error: {0}")]
    Model(#[from] models::Error),
    #[error("Discord error: {0}")]
    Discord(#[from] SerenityError),
}

pub type Result<T> = std::result::Result<T, Error>;
