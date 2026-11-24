use super::{Context, Result, guild_id};
use crate::{bot::Data, lichess::Format, models::User};
use poise::{
    CreateReply,
    serenity_prelude::{CreateEmbed, CreateEmbedFooter, GuildId, Http, RoleId, UserId},
};
use strum::IntoEnumIterator;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Retrieves your updated lichess ratings and gives you Discord roles
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn rating(ctx: Context<'_>) -> Result<()> {
    let reply = update_ratings(ctx.data(), ctx.http(), guild_id(ctx)?, ctx.author().id).await?;
    ctx.send(reply).await?;
    Ok(())
}

async fn update_rating_roles(
    http: &Http,
    guild_id: GuildId,
    discord_id: UserId,
    rating_roles: Vec<RoleId>,
    removeable_roles: Vec<RoleId>,
) -> Result<(Vec<RoleId>, Vec<RoleId>)> {
    trace!("update_rating_roles() called");
    let member = http.get_member(guild_id, discord_id).await.map_err(|e| {
        error!(
            "Could not retrieve user information for discord_id={} in guild_id={}: {}",
            discord_id, guild_id, e
        );
        e
    })?;

    let mut added = vec![];
    let mut removed = vec![];

    for role_id in rating_roles {
        if member.roles.contains(&role_id) {
            debug!("User already has role_id={}", role_id)
        } else {
            debug!("User is missing role_id={}", role_id);
            http.add_member_role(guild_id, discord_id, role_id, None)
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
        if member.roles.contains(&role_id) {
            debug!("User has extra role_id={} that should be removed", role_id);
            http.remove_member_role(guild_id, discord_id, role_id, None)
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

async fn update_ratings(
    data: &Data,
    http: &Http,
    guild_id: GuildId,
    discord_id: UserId,
) -> Result<CreateReply> {
    trace!("update_ratings() called");

    info!(
        "Updating ratings for discord_id={} in guild_id={}",
        discord_id, guild_id
    );

    let Data {
        pool,
        lichess,
        roles: rm,
    } = data;
    match User::find(pool, guild_id, discord_id).await? {
        Some(mut user) => {
            let old_ratings = user.get_ratings().clone();
            let ratings = user.update_ratings(pool, lichess).await?;

            let rating_roles = rm.find_rating_range_roles(guild_id, ratings);
            let removeable_roles = rm.other_rating_range_roles(guild_id, &rating_roles);
            let (added, removed) =
                update_rating_roles(http, guild_id, discord_id, rating_roles, removeable_roles)
                    .await?;

            let mut embed = CreateEmbed::default();

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
                embed = embed.field(format.to_string(), description, true);
            }

            if !added.is_empty() {
                let role_names = rm.get_rating_role_names(guild_id, &added);
                embed = embed.field("Roles added", role_names.join(", "), false);
            }

            if !removed.is_empty() {
                let role_names = rm.get_rating_role_names(guild_id, &removed);
                embed = embed.field("Roles removed", role_names.join(", "), false);
            }

            embed = embed
                .description(format!(
                    "Ratings for [{}](https://lichess.org/@/{}) from \
                            [lichess](https://lichess.org).",
                    user.get_lichess_username(),
                    user.get_lichess_username()
                ))
                .footer(CreateEmbedFooter::new(format!(
                    "Liro version {}. Please note that this bot only cares about the four rating formats shown above. \
                     Provisional ratings are ignored.",
                    VERSION
                )));

            Ok(CreateReply::default().embed(embed))
        }
        None => Ok(CreateReply::default().content(
            "Couldn't find a lichess user associated with your account. Please use the `ohnomy \
            account` (or `/link`) command first.",
        )),
    }
}
