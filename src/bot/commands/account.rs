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
    trace!("account() called");
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
    let challenge = Challenge::new(&pool, discord_id).await?;

    let whisper = format!(
        "Please connect your account using the following link: {}",
        challenge.lichess_url()
    );

    let message = match msg
        .author
        .dm(&ctx, |m| {
            m.content(whisper);
            m
        })
        .await
    {
        Ok(_) => "Please check your DMs :)",
        Err(e) => {
            warn!("Failed to send DM to user {}: {}", discord_id, e);
            "I wasn't able to send you a DM. Could you please allow me to message you so I can verify your lichess account?"
        }
    };

    msg.channel_id
        .send_message(&ctx, |m| {
            m.content(message);
            m
        })
        .await?;

    Ok(())
}

async fn update_rating_roles(
    ctx: &Context,
    guild_id: u64,
    author: &serenity::model::prelude::User,
    rating: i16,
) -> CommandResult {
    trace!("update_rating_roles() called");
    let discord_id = *author.id.as_u64();
    let member = ctx.http.get_member(guild_id, discord_id).await?;

    let role_id;
    let unneeded_roles;
    {
        let data = ctx.data.read().await;
        let role_manager = data.get::<GuildRoleManagerContainer>().unwrap().clone();
        role_id = role_manager
            .find_rating_range_role(guild_id, rating)
            .unwrap();
        unneeded_roles = role_manager.other_rating_range_roles(guild_id, role_id);
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
    trace!("rating() called");
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
                Some(old_rating) => {
                    let response = if old_rating == rating {
                        format!("Your average lichess rating is still {}", rating)
                    } else {
                        user.update_rating(&pool, rating).await?;

                        if old_rating > rating {
                            format!(
                                ":chart_with_downwards_trend: Your average lichess rating went \
                                down from {} to {}",
                                old_rating, rating
                            )
                        } else {
                            format!(
                                ":chart_with_upwards_trend: Your average lichess rating went up \
                                from {} to {}",
                                old_rating, rating
                            )
                        }
                    };
                    msg.channel_id
                        .send_message(&ctx, |m| {
                            m.content(response);
                            m
                        })
                        .await?;
                }
                None => {
                    user.update_rating(&pool, rating).await?;
                    msg.channel_id
                        .send_message(&ctx, |m| {
                            m.content(format!(
                                "Welcome to the liro gang! Your average lichess rating is {}",
                                rating
                            ));
                            m
                        })
                        .await?;
                }
            }

            Ok(())
        }
        Ok(None) => {
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.content(
                        "Couldn't find a lichess user associated with your account. \
                        Please use the `ohnomy account` command first.",
                    );
                    m
                })
                .await?;
            Ok(())
        }
        Err(why) => {
            error!("Unable to query database: {}", why);
            Ok(())
        }
    }
}
