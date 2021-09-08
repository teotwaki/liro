use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::*,
    prelude::*,
};

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    trace!("ping() called");
    msg.channel_id.say(&ctx.http, "Pong!").await?;

    Ok(())
}

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    trace!("help() called");
    let message = "Hi, I'm liro!\n\
                   I help automate role assignments based on your rating. To get started, please \
                   link your lichess account to your Discord user by saying `ohnomy \
                   account`\n\
                   After that, you can ask me to calculate your average rating and update your \
                   Discord role if necessary by saying `ohnomy rating`";
    msg.channel_id.say(&ctx.http, message).await?;

    Ok(())
}
