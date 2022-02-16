use super::{Format, Result};
use crate::config;
use reqwest::header;
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

        let mut headers = header::HeaderMap::new();

        if let Some(token) = config::lichess_token() {
            match header::HeaderValue::from_str(&token) {
                Ok(mut auth_value) => {
                    auth_value.set_sensitive(true);

                    headers.insert(header::AUTHORIZATION, auth_value);
                }
                Err(why) => error!("Invalid header value: {}", why),
            }
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Client { http }
    }

    pub async fn validate_token<T>(&self, access_token: T) -> Result<LichessUser>
    where
        T: AsRef<str>,
    {
        trace!("Client::validate_token() called");

        let result = self
            .http
            .get("https://lichess.org/api/account")
            .header("Authorization", format!("Bearer {}", access_token.as_ref()))
            .send()
            .await?;

        Ok(result.json::<LichessUser>().await?)
    }

    pub async fn fetch_user_ratings<U>(&self, username: U) -> Result<HashMap<Format, i16>>
    where
        U: AsRef<str>,
    {
        trace!("Client::fetch_user_ratings() called");
        let url = format!("https://lichess.org/api/user/{}", username.as_ref());
        let profile = self.http.get(url).send().await?.json::<Profile>().await?;
        Ok(profile.get_ratings())
    }

    pub async fn fetch_access_token<C, V>(&self, code: C, code_verifier: V) -> Result<String>
    where
        C: AsRef<str>,
        V: AsRef<str>,
    {
        trace!("Client::fetch_access_token() called");
        let query_params = [
            ("grant_type", "authorization_code"),
            ("code", code.as_ref()),
            ("code_verifier", code_verifier.as_ref()),
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
