use crate::{
    bot::run::PoolContainer,
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
