use super::{Format, Result};
use crate::config;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct LichessUser {
    username: String,
    title: Option<String>,
}

impl LichessUser {
    pub fn get_username(&self) -> &str {
        &self.username
    }

    pub fn is_bot(&self) -> bool {
        self.title == Some("BOT".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AccessToken {
    pub access_token: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct FormatRating {
    games: Option<i64>,
    rating: Option<i16>,
    prov: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    perfs: HashMap<String, FormatRating>,
}

impl Profile {
    pub fn get_ratings(&self) -> HashMap<Format, i16> {
        trace!("Profile::get_ratings_for() called");
        self.perfs
            .iter()
            .filter_map(|(k, v)| match (v.prov, v.rating, k.parse::<Format>()) {
                (None, Some(rating), Ok(format)) => Some((format, rating)),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    pub fn new() -> Client {
        trace!("Client::new() called");

        Client {
            http: reqwest::Client::new(),
        }
    }

    pub async fn validate_token(&self, access_token: &str) -> Result<LichessUser> {
        trace!("Client::validate_token() called");

        let result = self
            .http
            .get("https://lichess.org/api/account")
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        Ok(result.json::<LichessUser>().await?)
    }

    pub async fn fetch_user_ratings(&self, username: &str) -> Result<HashMap<Format, i16>> {
        trace!("Client::fetch_user_ratings() called");
        let url = format!("https://lichess.org/api/user/{}", username);
        let profile = self.http.get(url).send().await?.json::<Profile>().await?;
        Ok(profile.get_ratings())
    }

    pub async fn fetch_access_token(&self, code: &str, code_verifier: &str) -> Result<String> {
        trace!("Client::fetch_access_token() called");
        let query_params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", code_verifier),
            ("client_id", &config::client_id()),
            (
                "redirect_uri",
                &format!("{}/oauth/callback", config::hostname()),
            ),
        ];

        let parsed = self
            .http
            .post("https://lichess.org/api/token")
            .form(&query_params)
            .send()
            .await?
            .json::<AccessToken>()
            .await?;

        Ok(parsed.access_token)
    }
}
