use crate::{
    bot::run::{GuildRoleManagerContainer, PoolContainer},
    lichess,
    models::{Challenge, User},
};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
async fn account(ctx: &Context, msg: &Message) -> CommandResult {
    let discord_id = *msg.author.id.as_u64();
    let pool;
    {
        let data = ctx.data.read().await;
        pool = data.get::<PoolContainer>().unwrap().clone();
    }
    let challenge = Challenge::new(&pool, discord_id).await?;

    let whisper = format!(
        "Please connect your account using the following link: {}",
        challenge.link()
    );

    let dm_future = msg.author.dm(&ctx, |m| {
        m.content(whisper);
        m
    });

    let reply_future = msg.channel_id.send_message(&ctx, |m| {
        m.content("Please check your DMs :)");
        m
    });

    match tokio::join!(dm_future, reply_future) {
        (Err(e), _) => Err(Box::new(e)),
        (_, Err(e)) => Err(Box::new(e)),
        _ => Ok(()),
    }
}

async fn update_rating_roles(
    ctx: &Context,
    guild_id: u64,
    author: &serenity::model::prelude::User,
    rating: i16,
) -> CommandResult {
    let discord_id = *author.id.as_u64();
    let member = ctx.http.get_member(guild_id, discord_id).await?;

    let role_id;
    let unneeded_roles;
    {
        let data = ctx.data.read().await;
        let role_manager = data.get::<GuildRoleManagerContainer>().unwrap().clone();
        let dg = role_manager.lock().await.clone();
        role_id = dg.find_rating_range_role(guild_id, rating).unwrap();
        unneeded_roles = dg.other_rating_range_roles(guild_id, role_id);
    }

    debug!("Found role for rating level: {}", role_id);

    if member.roles.contains(&RoleId(role_id)) {
        debug!("User already has correct role");
    } else {
        debug!("User is missing the role, adding");
        ctx.http
            .add_member_role(guild_id, discord_id, role_id)
            .await
            .unwrap();
        debug!("User role added");
    }

    for role_id in unneeded_roles {
        if member.roles.contains(&RoleId(role_id)) {
            debug!("User has extra role that should be removed");
            ctx.http
                .remove_member_role(guild_id, discord_id, role_id)
                .await?;
            debug!("Role removed");
        }
    }

    Ok(())
}

#[command]
async fn rating(ctx: &Context, msg: &Message) -> CommandResult {
    let discord_id = *msg.author.id.as_u64();
    debug!(
        "Handling rating command for user with discord_id={}",
        discord_id
    );

    let pool;
    {
        let data = ctx.data.read().await;
        pool = data.get::<PoolContainer>().unwrap().clone();
    }

    match User::find(&pool, discord_id).await {
        Ok(Some(mut user)) => {
            let old_rating = user.rating();
            let rating = lichess::api::fetch_user_rating(&user.lichess_username()).await?;
            update_rating_roles(ctx, *msg.guild_id.unwrap().as_u64(), &msg.author, rating).await?;
            match old_rating {
                Some(old_rating) if old_rating == rating => {
                    msg.channel_id
                        .send_message(&ctx, |m| {
                            m.content(format!("Sorry, no change. Your rating is still {}", rating));
                            m
                        })
                        .await?;
                }
                _ => {
                    user.update_rating(&pool, rating).await?;
                    msg.channel_id
                        .send_message(&ctx, |m| {
                            m.content(format!("Your rating is now {}", rating));
                            m
                        })
                        .await?;
                }
            }
            Ok(())
        }
        Ok(None) => {
            msg.channel_id.send_message(&ctx, |m| {
                m.content("Couldn't find a lichess user associated with your account. Please use the `account` command first.");
                m
            }).await?;
            Ok(())
        }
        Err(why) => {
            error!("Unable to query database: {}", why);
            Ok(())
        }
    }
}
