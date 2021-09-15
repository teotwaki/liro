use super::{Error, Result};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct LichessUser {
    username: String,
}

impl LichessUser {
    pub fn get_username(&self) -> &str {
        trace!("LichessUser::get_username() called");
        &self.username
    }
}

pub async fn fetch_account(access_token: &str) -> Result<LichessUser> {
    trace!("fetch_account() called");
    let result = reqwest::Client::new()
        .get("https://lichess.org/api/account")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if result.status().is_success() {
        Ok(serde_json::from_str::<LichessUser>(
            &result.text().await.unwrap(),
        )?)
    } else {
        Err(Error::UnexpectedStatus)
    }
}

#[derive(Deserialize)]
struct FormatRating {
    games: Option<i64>,
    rating: Option<i16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    perfs: HashMap<String, FormatRating>,
}

impl Profile {
    pub fn calculate_rating(&self) -> i16 {
        trace!("Profile::calculate_rating() called");
        let game_modes: Vec<String> = ["bullet", "blitz", "rapid", "classical"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let (total_rating, total_games) = self.perfs.iter().fold((0, 0), |acc, (k, v)| {
            match (v.rating, v.games, game_modes.contains(k)) {
                (Some(rating), Some(games), true) => {
                    let rating = acc.0 + rating as i64 * games;
                    let games = acc.1 + games;

                    (rating, games)
                }
                _ => acc,
            }
        });

        (total_rating / total_games) as i16
    }
}

pub async fn fetch_user_rating(user: &str) -> Result<i16> {
    trace!("fetch_user_rating() called");
    let result = reqwest::get(format!("https://lichess.org/api/user/{}", user)).await?;

    if result.status().is_success() {
        let profile = serde_json::from_str::<Profile>(&result.text().await.unwrap())?;

        Ok(profile.calculate_rating())
    } else {
        Err(Error::UnexpectedStatus)
    }
}
