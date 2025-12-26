use super::{
    commands::{account::*, meta::*},
    role_manager::RoleManager,
};
use crate::{bot::Handler, db::Pool, lichess};
use futures::join;
use serenity::{
    all::{ShardManager, standard::Configuration},
    framework::{
        StandardFramework,
        standard::macros::{group, hook},
    },
    http::Http,
    model::channel::Message,
    prelude::*,
};
use std::{env, sync::Arc};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

pub struct PoolContainer;

impl TypeMapKey for PoolContainer {
    type Value = Pool;
}

pub struct RoleManagerContainer;

impl TypeMapKey for RoleManagerContainer {
    type Value = RoleManager;
}

pub struct LichessClientContainer;

impl TypeMapKey for LichessClientContainer {
    type Value = lichess::Client;
}

#[group]
#[commands(help, account, rating, gdpr)]
struct General;

#[hook]
async fn unknown_command(ctx: &Context, msg: &Message, unknown_command_name: &str) {
    trace!("unknown_command() called");

    let message = format!(
        "Could not understand command `{}`. Please see `ohnomy help` for more information",
        unknown_command_name
    );
    if let Err(e) = msg.channel_id.say(&ctx.http, message).await {
        error!("Unable to send response to channel: {}", e);
    }
}

pub async fn run(pool: &Pool, lichess: &lichess::Client) {
    trace!("run() called");

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id = env::var("APPLICATION_ID")
        .expect("Expected to find the APPLICATION_ID environment variable")
        .parse()
        .expect("Expected the APPLICATION_ID environment variable to be an integer");

    let http = Http::new(&token);

    let (current_user, current_application) =
        join!(http.get_current_user(), http.get_current_application_info());

    let bot_id = current_user.expect("Could not access user info").id;

    // We will fetch your bot's owners and id
    let owners = current_application
        .map(|info| std::iter::once(info.owner.unwrap().id).collect())
        .unwrap_or_else(|why| panic!("Could not access application info: {:?}", why));

    // Create the framework
    let mut framework = StandardFramework::new();
    framework.configure(
        Configuration::new()
            .owners(owners)
            .with_whitespace(true)
            .prefix("") // disable default ~ prefix
            .prefixes(vec!["ohnomy", "oh no my"])
            .case_insensitivity(true)
            .on_mention(Some(bot_id))
            .ignore_bots(true),
    );

    framework = framework
        .unrecognised_command(unknown_command)
        .group(&GENERAL_GROUP);

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let intents = GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(Handler {})
        .application_id(application_id)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<PoolContainer>(pool.clone());
        data.insert::<RoleManagerContainer>(RoleManager::new());
        data.insert::<LichessClientContainer>(lichess.clone());
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.shutdown_all().await;
    });

    info!("Starting bot");

    match client.start().await {
        Ok(_) => info!("Bot shutting down"),
        Err(why) => error!("Bot returned an error: {:?}", why),
    }
}
