use super::error::{Error, Result};
use crate::{
    db::Pool,
    lichess,
    models::{Challenge, Guild, User},
};
use askama::Template;
use serde::Deserialize;
use warp::Reply;

#[derive(Template)]
#[template(path = "invited.html")]
struct BotInvitedTemplate;

#[derive(Template)]
#[template(path = "linked.html")]
struct AccountLinkedTemplate<'a> {
    username: &'a str,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    guild_count: usize,
    user_count: usize,
    unique_user_count: usize,
    challenge_count: usize,
}

#[derive(Deserialize, Debug)]
pub struct CallbackParams {
    code: String,
    state: u64,
}

pub async fn bot_invited_handler() -> Result<impl Reply> {
    trace!("bot_invited_handler() called");

    let template = BotInvitedTemplate {};

    match template.render() {
        Ok(output) => Ok(warp::reply::html(output)),
        Err(e) => Err(warp::reject::custom(Error::Template(e))),
    }
}

pub async fn oauth_callback_handler(
    params: CallbackParams,
    pool: Pool,
    lichess: lichess::Client,
) -> Result<impl Reply> {
    trace!("oauth_callback_handler() called");
    let challenge = Challenge::find(&pool, params.state)
        .await
        .map_err(Error::Database)?
        .ok_or(Error::ChallengeNotFound)?;

    let access_token = lichess
        .fetch_access_token(&params.code, &challenge.code_verifier())
        .await
        .map_err(Error::Lichess)?;

    let lichess_user = lichess
        .validate_token(&access_token)
        .await
        .map_err(Error::Lichess)?;

    if lichess_user.is_bot() {
        return Err(Error::BotAccount.into());
    }

    let username = lichess_user.get_username().to_string();

    let user = User::find_by_username(&pool, challenge.guild_id(), &username)
        .await
        .map_err(Error::Database)?;

    if user.is_some() {
        return Err(Error::DuplicateLink.into());
    }

    let user = User::new(
        &pool,
        challenge.guild_id(),
        challenge.discord_id(),
        username,
    )
    .await
    .map_err(Error::Database)?;

    challenge.delete(&pool).await.map_err(Error::Database)?;

    let template = AccountLinkedTemplate {
        username: user.get_lichess_username(),
    };

    match template.render() {
        Ok(output) => Ok(warp::reply::html(output)),
        Err(e) => Err(warp::reject::custom(Error::Template(e))),
    }
}

pub async fn dashboard_handler(pool: Pool) -> Result<impl Reply> {
    trace!("dashboard_handler() called");

    let (guild_count, user_count, unique_user_count, challenge_count) = tokio::join!(
        Guild::count(&pool),
        User::count(&pool),
        User::unique_count(&pool),
        Challenge::count(&pool)
    );

    let template = DashboardTemplate {
        guild_count: guild_count.map_err(Error::Database)?,
        user_count: user_count.map_err(Error::Database)?,
        unique_user_count: unique_user_count.map_err(Error::Database)?,
        challenge_count: challenge_count.map_err(Error::Database)?,
    };

    match template.render() {
        Ok(output) => Ok(warp::reply::html(output)),
        Err(e) => Err(warp::reject::custom(Error::Template(e))),
    }
}
