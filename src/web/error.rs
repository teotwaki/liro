use crate::{lichess, models};
use askama::Template;
use std::convert::Infallible;
use thiserror::Error;
use warp::{http::StatusCode, reply, reply::html, Rejection, Reply};

#[derive(Error, Debug)]
pub enum Error {
    #[error("error accessing database: {0}")]
    Database(#[from] models::Error),
    #[error("authentication challenge not found")]
    ChallengeNotFound,
    #[error("The account you are trying to link is already in use on this Discord server")]
    DuplicateLink,
    #[error("bot accounts are not allowed")]
    BotAccount,
    #[error("templating error: {0}")]
    Template(#[from] askama::Error),
    #[error("lichess error: {0}")]
    Lichess(#[from] lichess::Error),
}

impl warp::reject::Reject for Error {}

pub type Result<T> = std::result::Result<T, Rejection>;

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate<'a> {
    message: &'a str,
}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    trace!("handle_rejection() called");
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not Found".to_string();
    } else if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        code = StatusCode::BAD_REQUEST;
        message = "Invalid Body".to_string();
    } else if let Some(e) = err.find::<Error>() {
        match e {
            Error::ChallengeNotFound => {
                code = StatusCode::NOT_FOUND;
                message = e.to_string();
            }
            Error::DuplicateLink | Error::BotAccount => {
                code = StatusCode::CONFLICT;
                message = e.to_string();
            }
            _ => {
                error!("unhandled application error: {:?}", err);
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal Server Error".to_string();
            }
        }
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Method Not Allowed".to_string();
    } else {
        error!("unhandled error: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error".to_string();
    }

    let template = ErrorTemplate { message: &message };

    match template.render() {
        Ok(v) => Ok(reply::with_status(html(v), code)),
        Err(_) => Ok(reply::with_status(
            html(message.to_string()),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
