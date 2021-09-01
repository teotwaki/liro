use super::{Error::*, Result};
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
    let query_params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", code_verifier),
        ("client_id", "liro-bot-test"),
        (
            "redirect_uri",
            &format!("{}/oauth/callback", "http://localhost:8000"),
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
