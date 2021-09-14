use super::{Error::*, Result};
use crate::config;
use serde::Deserialize;

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
}

impl AccessToken {
    pub fn get(&self) -> &str {
        &self.access_token
    }
}

pub async fn fetch_access_token(code: &str, code_verifier: &str) -> Result<String> {
    trace!("fetch_access_token() called");
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

    let result = reqwest::Client::new()
        .post("https://lichess.org/api/token")
        .form(&query_params)
        .send()
        .await?;

    if result.status().is_success() {
        let at: AccessToken = serde_json::from_str(&result.text().await.unwrap())?;
        Ok(at.get().to_string())
    } else {
        Err(UnexpectedStatusError)
    }
}

pub fn oauth_url(code_challenge: &str, state: &str) -> String {
    format!(
        "https://lichess.org/oauth\
             ?response_type=code\
             &redirect_uri={}/oauth/callback\
             &client_id={}\
             &code_challenge_method=S256\
             &code_challenge={}\
             &state={}",
        config::hostname(),
        config::client_id(),
        code_challenge,
        state
    )
}
