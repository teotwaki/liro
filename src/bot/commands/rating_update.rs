use super::{Response, Result};
use crate::{
    bot::run::{LichessClientContainer, PoolContainer, RoleManagerContainer},
    lichess::Format,
    models::User,
};
use serenity::{builder::CreateEmbed, model::prelude::*, prelude::*};
use strum::IntoEnumIterator;

const VERSION: &str = env!("CARGO_PKG_VERSION");

async fn update_rating_roles(
    ctx: &Context,
    guild_id: u64,
    discord_id: u64,
    rating_roles: Vec<u64>,
    removeable_roles: Vec<u64>,
) -> Result<(Vec<u64>, Vec<u64>)> {
    trace!("update_rating_roles() called");
    let member = ctx
        .http
        .get_member(guild_id, discord_id)
        .await
        .map_err(|e| {
            error!(
                "Could not retrieve user information for discord_id={} in guild_id={}: {}",
                discord_id, guild_id, e
            );
            e
        })?;

    let mut added = vec![];
    let mut removed = vec![];

    for role_id in rating_roles {
        if member.roles.contains(&RoleId(role_id)) {
            debug!("User already has role_id={}", role_id)
        } else {
            debug!("User is missing role_id={}", role_id);
            ctx.http
                .add_member_role(guild_id, discord_id, role_id)
                .await
                .map_err(|e| {
                    error!(
                        "Could not add role_id={} to discord_id={}: {}",
                        role_id, discord_id, e
                    );
                    e
                })?;
            added.push(role_id);
            debug!("Added role_id={} to discord_id={}", role_id, discord_id);
        }
    }

    for role_id in removeable_roles {
        if member.roles.contains(&RoleId(role_id)) {
            debug!("User has extra role_id={} that should be removed", role_id);
            ctx.http
                .remove_member_role(guild_id, discord_id, role_id)
                .await
                .map_err(|e| {
                    error!(
                        "Could not remove role_id={} from discord_id={}: {}",
                        role_id, discord_id, e
                    );
                    e
                })?;
            removed.push(role_id);
            debug!("Removed role_id={} from discord_id={}", role_id, discord_id);
        }
    }

    Ok((added, removed))
}

pub async fn update_ratings(ctx: &Context, guild_id: u64, discord_id: u64) -> Result<Response> {
    trace!("update_ratings() called");

    info!(
        "Updating ratings for discord_id={} in guild_id={}",
        discord_id, guild_id
    );

    let lichess;
    let pool;
    let rm;
    {
        let data = ctx.data.read().await;

        lichess = data.get::<LichessClientContainer>().unwrap().clone();
        pool = data.get::<PoolContainer>().unwrap().clone();
        rm = data.get::<RoleManagerContainer>().unwrap().clone();
    }

    match User::find(&pool, guild_id, discord_id).await {
        Ok(Some(mut user)) => {
            let old_ratings = user.get_ratings().clone();
            let ratings = user.update_ratings(&pool, &lichess).await?.clone();

            let rating_roles = rm.find_rating_range_roles(guild_id, &ratings);
            let removeable_roles = rm.other_rating_range_roles(guild_id, &rating_roles);
            let (added, removed) =
                update_rating_roles(ctx, guild_id, discord_id, rating_roles, removeable_roles)
                    .await?;

            let mut embed = CreateEmbed {
                ..Default::default()
            };

            for format in Format::iter() {
                let old_rating = old_ratings.get(&format);
                let new_rating = ratings.get(&format);
                let description = match (old_rating, new_rating) {
                    (Some(old_rating), Some(new_rating)) if old_rating == new_rating => {
                        old_rating.to_string()
                    }
                    (Some(old_rating), Some(new_rating)) if old_rating < new_rating => {
                        format!(
                            ":chart_with_upwards_trend: {} -> {}",
                            old_rating, new_rating
                        )
                    }
                    (Some(old_rating), Some(new_rating)) if old_rating > new_rating => {
                        format!(
                            ":chart_with_downwards_trend: {} -> {}",
                            old_rating, new_rating
                        )
                    }
                    (None, Some(new_rating)) => {
                        format!(":new: {}", new_rating)
                    }
                    (Some(old_rating), None) => {
                        format!(":crying_cat_face: ~~{}~~", old_rating)
                    }
                    _ => "Unrated (or provisional)".to_string(),
                };
                embed.field(format.to_string(), description, true);
            }

            if !added.is_empty() {
                let role_names = rm.get_rating_role_names(guild_id, &added);
                embed.field("Roles added", role_names.join(", "), false);
            }

            if !removed.is_empty() {
                let role_names = rm.get_rating_role_names(guild_id, &removed);
                embed.field("Roles removed", role_names.join(", "), false);
            }

            embed
                .description(format!(
                    "Ratings for [{}](https://lichess.org/@/{}) from \
                            [lichess](https://lichess.org).",
                    user.get_lichess_username(),
                    user.get_lichess_username()
                ))
                .footer(|f| {
                    f.text(format!(
                        "Liro version {}. Please note that this bot only cares about the \
                                four rating formats shown above. Provisional ratings are ignored.",
                        VERSION
                    ))
                });

            Ok(Response::Embed(embed))
        }
        Ok(None) => Ok(Response::Sentence(
            "Couldn't find a lichess user associated with your account. Please use the `ohnomy \
            account` (or `/link`) command first."
                .to_string(),
        )),
        Err(why) => {
            error!("Unable to query database: {}", why);
            Ok(Response::Sentence(
                "Internal bot error. @teotwaki, I'm scared.".to_string(),
            ))
        }
    }
}
