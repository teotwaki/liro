use crate::{
    bot::run::{LichessClientContainer, PoolContainer, RoleManagerContainer},
    lichess,
    models::{Challenge, User},
};
use futures::future;
use serenity::{
    framework::standard::{macros::command, CommandError, CommandResult},
    model::prelude::*,
    prelude::*,
};
use strum::IntoEnumIterator;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[command]
async fn gdpr(ctx: &Context, msg: &Message) -> CommandResult {
    trace!("gdpr() called");
    let guild_id = *msg.guild_id.unwrap().as_u64();
    let discord_id = *msg.author.id.as_u64();

    info!(
        "Deleting data for discord_id={} in guild_id={}",
        discord_id, guild_id,
    );

    let pool;
    let rm;
    {
        let data = ctx.data.read().await;
        pool = data.get::<PoolContainer>().unwrap().clone();
        rm = data.get::<RoleManagerContainer>().unwrap().clone();
    }

    match User::find(&pool, guild_id, discord_id).await {
        Ok(Some(mut user)) => {
            let member = ctx.http.get_member(guild_id, discord_id).await?;
            let role_ids = rm.other_rating_range_roles(guild_id, &[]);
            let mut futures = vec![];

            for role_id in role_ids {
                if member.roles.contains(&RoleId(role_id)) {
                    futures.push(ctx.http.remove_member_role(guild_id, discord_id, role_id));
                }
            }

            for res in future::join_all(futures).await {
                if let Err(e) = res {
                    error!("Couldn't remove role from discord_id={}: {}", discord_id, e);
                }
            }

            user.delete(&pool).await?;

            msg.channel_id
                .send_message(&ctx, |m| {
                    m.content("User information deleted. Toodles! :wave:")
                })
                .await?;
        }
        Ok(None) => {
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.content("I don't see any data to delete:question:")
                })
                .await?;
        }
        Err(why) => {
            error!("Unable to query database: {}", why);
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.content("Internal bot error. @teotwaki, I'm scared.")
                })
                .await?;
        }
    }

    Ok(())
}

#[command]
async fn account(ctx: &Context, msg: &Message) -> CommandResult {
    trace!("account() called");
    let guild_id = *msg.guild_id.unwrap().as_u64();
    let discord_id = *msg.author.id.as_u64();

    info!(
        "Handling account command for {} (id={})",
        msg.author.name, discord_id
    );
    let pool;
    {
        let data = ctx.data.read().await;
        pool = data.get::<PoolContainer>().unwrap().clone();
    }
    let challenge = Challenge::new(&pool, guild_id, discord_id).await?;

    let whisper = format!(
        "Please connect your account using the following link: {}",
        challenge.lichess_url()
    );

    let message = match msg.author.dm(&ctx, |m| m.content(whisper)).await {
        Ok(_) => "Please check your DMs :)",
        Err(e) => {
            warn!("Failed to send DM to user {}: {}", discord_id, e);
            "I wasn't able to send you a DM. Could you please allow me to message you so I can verify your lichess account?"
        }
    };

    msg.channel_id
        .send_message(&ctx, |m| m.content(message))
        .await?;

    Ok(())
}

async fn update_rating_roles(
    ctx: &Context,
    msg: &Message,
    rating_roles: Vec<u64>,
    removeable_roles: Vec<u64>,
) -> Result<(Vec<u64>, Vec<u64>), CommandError> {
    trace!("update_rating_roles() called");
    let discord_id = *msg.author.id.as_u64();
    let guild_id = *msg.guild_id.unwrap().as_u64();
    let member = ctx.http.get_member(guild_id, discord_id).await?;

    let mut added = vec![];
    let mut add_futures = vec![];
    let mut removed = vec![];
    let mut rem_futures = vec![];

    for role_id in rating_roles {
        if member.roles.contains(&RoleId(role_id)) {
            debug!("User already has role_id={}", role_id)
        } else {
            debug!("User is missing role_id={}", role_id);
            add_futures.push(ctx.http.add_member_role(guild_id, discord_id, role_id));
            added.push(role_id);
            debug!("Added role_id={} to discord_id={}", role_id, discord_id);
        }
    }

    for res in future::join_all(add_futures).await {
        if let Err(e) = res {
            error!("Failed to add role to discord_id={}: {}", discord_id, e);
        }
    }

    for role_id in removeable_roles {
        if member.roles.contains(&RoleId(role_id)) {
            debug!("User has extra role_id={} that should be removed", role_id);
            rem_futures.push(ctx.http.remove_member_role(guild_id, discord_id, role_id));
            removed.push(role_id);
            debug!("Removed role_id={} from discord_id={}", role_id, discord_id);
        }
    }

    for res in future::join_all(rem_futures).await {
        if let Err(e) = res {
            error!(
                "Failed to remove role from discord_id={}: {}",
                discord_id, e
            );
        }
    }

    Ok((added, removed))
}

#[command]
async fn rating(ctx: &Context, msg: &Message) -> CommandResult {
    trace!("rating() called");
    let guild_id = *msg.guild_id.unwrap().as_u64();
    let discord_id = *msg.author.id.as_u64();
    debug!(
        "Handling rating command for user with discord_id={}",
        discord_id
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
                update_rating_roles(ctx, msg, rating_roles, removeable_roles).await?;

            msg.channel_id
                .send_message(&ctx, |m| {
                    m.add_embed(|e| {
                        for format in lichess::Format::iter() {
                            let old_rating = old_ratings.get(&format);
                            let new_rating = ratings.get(&format);
                            let description = match (old_rating, new_rating) {
                                (Some(old_rating), Some(new_rating))
                                    if old_rating == new_rating =>
                                {
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
                            e.field(format.to_string(), description, true);
                        }

                        if !added.is_empty() {
                            let role_names = rm.get_rating_role_names(guild_id, &added);
                            e.field("Roles added", role_names.join(", "), false);
                        }

                        if !removed.is_empty() {
                            let role_names = rm.get_rating_role_names(guild_id, &removed);
                            e.field("Roles removed", role_names.join(", "), false);
                        }

                        e.description(format!(
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
                        })
                    })
                })
                .await?;
        }
        Ok(None) => {
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.content(
                        "Couldn't find a lichess user associated with your account. \
                        Please use the `ohnomy account` command first.",
                    )
                })
                .await?;
        }
        Err(why) => {
            error!("Unable to query database: {}", why);
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.content("Internal bot error. @teotwaki, I'm scared.")
                })
                .await?;
        }
    };

    Ok(())
}
