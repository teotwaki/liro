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
    guilds: usize,
    users: usize,
    challenges: usize,
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
        .map_err(|_| Error::DBAccess)?
        .ok_or(Error::ChallengeNotFound)?;

    let access_token = lichess
        .fetch_access_token(&params.code, &challenge.code_verifier())
        .await
        .map_err(Error::Lichess)?;

    let username = lichess
        .validate_token(&access_token)
        .await
        .map_err(Error::Lichess)?;

    let user = User::new(
        &pool,
        challenge.guild_id(),
        challenge.discord_id(),
        username.to_string(),
    )
    .await
    .map_err(|_| Error::DBAccess)?;

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

    let (guilds, users, challenges) = tokio::join!(
        Guild::count(&pool),
        User::count(&pool),
        Challenge::count(&pool)
    );

    let template = DashboardTemplate {
        guilds: guilds.map_err(|_| Error::DBAccess)?,
        users: users.map_err(|_| Error::DBAccess)?,
        challenges: challenges.map_err(|_| Error::DBAccess)?,
    };

    match template.render() {
        Ok(output) => Ok(warp::reply::html(output)),
        Err(e) => Err(warp::reject::custom(Error::Template(e))),
    }
}
