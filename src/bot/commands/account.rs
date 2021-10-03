use super::rating_update::{update_ratings, Response as RatingUpdateResponse};
use crate::{
    bot::run::{PoolContainer, RoleManagerContainer},
    models::{Challenge, User},
};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::*,
    prelude::*,
};

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
            let role_ids = rm.other_rating_range_roles(guild_id, &[]);

            for role_id in role_ids {
                if member.roles.contains(&RoleId(role_id)) {
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

#[command]
async fn rating(ctx: &Context, msg: &Message) -> CommandResult {
    trace!("rating() called");
    let guild_id = *msg.guild_id.unwrap().as_u64();
    let discord_id = *msg.author.id.as_u64();
    debug!(
        "Handling rating command for user with discord_id={}",
        discord_id
    );

    match update_ratings(ctx, guild_id, discord_id).await? {
        RatingUpdateResponse::Embed(e) => {
            msg.channel_id
                .send_message(&ctx, |m| m.set_embed(e))
                .await?
        }
        RatingUpdateResponse::Sentence(s) => {
            msg.channel_id.send_message(&ctx, |m| m.content(s)).await?
        }
    };

    Ok(())
}
