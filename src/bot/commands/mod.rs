mod account;
mod meta;
mod rating_update;
mod reply;

use super::Data;
use crate::{db, lichess, models};
use poise::serenity_prelude::{Error as SerenityError, GuildId, HttpError};
pub(super) use reply::send_command_error;
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

impl Error {
    // Response bodies and transport errors can contain OAuth data or interaction URLs.
    // Use only known categories and numeric codes when logging command failures.
    pub fn diagnostic(&self) -> String {
        let category = match self {
            Self::Discord(error) => return discord_diagnostic(error),
            Self::Model(error) => match error {
                models::Error::Serialization(_) => "stored data serialization error",
                models::Error::Database(db::Error::Client(_)) => "Redis client error",
                models::Error::Database(db::Error::Pool(_)) => "Redis connection pool error",
                models::Error::Database(db::Error::Command(_)) => "Redis command error",
                models::Error::Lichess(lichess::Error::Network(error)) => {
                    return format!("Lichess {}", request_failure(error));
                }
            },
            Self::GuildOnly => "missing guild context",
        };
        category.to_owned()
    }
}

fn discord_diagnostic(error: &SerenityError) -> String {
    match error {
        SerenityError::Http(HttpError::UnsuccessfulRequest(response)) => format!(
            "Discord HTTP {} (API error {})",
            response.status_code.as_u16(),
            response.error.code,
        ),
        SerenityError::Http(HttpError::Request(error)) => {
            format!("Discord {}", request_failure(error))
        }
        SerenityError::Http(_) => "Discord HTTP configuration or rate-limit error".to_owned(),
        SerenityError::Decode(..) | SerenityError::Json(_) => "Discord payload error".to_owned(),
        _ => "Discord client error".to_owned(),
    }
}

fn request_failure(error: &reqwest::Error) -> String {
    if let Some(status) = error.status() {
        format!("HTTP {}", status.as_u16())
    } else if error.is_timeout() {
        "request timed out".to_owned()
    } else if error.is_connect() {
        "connection failed".to_owned()
    } else if error.is_decode() {
        "response decoding failed".to_owned()
    } else {
        "request failed".to_owned()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_link_and_unlink_use_private_slash_responses() {
        let commands = registry();
        let slash: Vec<_> = commands
            .iter()
            .filter(|cmd| cmd.slash_action.is_some())
            .map(|cmd| (cmd.name.as_ref(), cmd.ephemeral))
            .collect();
        assert_eq!(slash, [("rating", false), ("link", true), ("unlink", true)]);
    }

    #[test]
    fn diagnostics_do_not_copy_raw_payloads_or_urls() {
        let json_error = serde_json::from_str::<u64>(r#""oauth-verifier""#).unwrap_err();
        assert!(json_error.to_string().contains("oauth-verifier"));
        let error = Error::Model(models::Error::Serialization(json_error));
        assert_eq!(error.diagnostic(), "stored data serialization error");

        let error = Error::from(SerenityError::Url(
            "https://discord.com/api/webhooks/1/interaction-token".to_owned(),
        ));
        assert_eq!(error.diagnostic(), "Discord client error");
    }
}
