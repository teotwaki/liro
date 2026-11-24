use super::{Context, Result, guild_id, reply::respond};
use crate::{
    bot::Data,
    db::Pool,
    models::{Challenge, User},
};
use poise::{
    CreateReply,
    serenity_prelude::{CreateMessage, GuildId, Http, UserId},
};

async fn unlink_account(
    data: &Data,
    http: &Http,
    guild_id: GuildId,
    discord_id: UserId,
) -> Result<&'static str> {
    info!("Deleting data for discord_id={discord_id} in guild_id={guild_id}");
    let Some(mut user) = User::find(&data.pool, guild_id, discord_id).await? else {
        return Ok("I don't see any data to delete:question:");
    };
    let member = http.get_member(guild_id, discord_id).await?;
    for role_id in data.roles.other_rating_range_roles(guild_id, []) {
        if member.roles.contains(&role_id) {
            http.remove_member_role(guild_id, discord_id, role_id, None)
                .await
                .map_err(|error| {
                    error!(
                        "Could not remove role_id={role_id} from discord_id={discord_id}: {error}"
                    );
                    error
                })?;
        }
    }
    user.delete(&data.pool).await?;
    Ok("User information deleted. Toodles! :wave:")
}

async fn create_link(pool: &Pool, guild_id: GuildId, discord_id: UserId) -> Result<String> {
    info!("Handling link command for discord_id={discord_id} in guild_id={guild_id}");
    let challenge = Challenge::new(pool, guild_id, discord_id).await?;
    Ok(format!(
        "Please connect your account using the following link: {}",
        challenge.lichess_url()
    ))
}

#[poise::command(prefix_command, guild_only)]
pub async fn gdpr(ctx: Context<'_>) -> Result<()> {
    let message = unlink_account(ctx.data(), ctx.http(), guild_id(ctx)?, ctx.author().id).await?;
    ctx.say(message).await?;
    Ok(())
}

/// Deletes all your information from the bot and removes your Discord roles.
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn unlink(ctx: Context<'_>) -> Result<()> {
    respond(ctx, async {
        let message =
            unlink_account(ctx.data(), ctx.http(), guild_id(ctx)?, ctx.author().id).await?;
        Ok(CreateReply::default().content(message))
    })
    .await
}

#[poise::command(prefix_command, guild_only)]
pub async fn account(ctx: Context<'_>) -> Result<()> {
    let discord_id = ctx.author().id;
    let whisper = create_link(&ctx.data().pool, guild_id(ctx)?, discord_id).await?;
    let message = match ctx
        .author()
        .dm(ctx.http(), CreateMessage::new().content(whisper))
        .await
    {
        Ok(_) => "Please check your DMs :)",
        Err(_) => {
            debug!("Failed to send DM to user {discord_id}");
            "I wasn't able to send you a DM. Could you please allow me to message you so I can verify your lichess account?"
        }
    };
    ctx.say(message).await?;
    Ok(())
}

/// Connects your lichess.org account with Liro. Needed to update ratings.
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn link(ctx: Context<'_>) -> Result<()> {
    respond(ctx, async {
        let whisper = create_link(&ctx.data().pool, guild_id(ctx)?, ctx.author().id).await?;
        Ok(CreateReply::default().content(whisper))
    })
    .await
}
