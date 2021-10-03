use super::{
    commands::{account::*, meta::*},
    role_manager::RoleManager,
};
use crate::{bot::Handler, db::Pool, lichess};
use serenity::{
    client::bridge::gateway::{GatewayIntents, ShardManager},
    framework::{
        standard::macros::{group, hook},
        StandardFramework,
    },
    http::Http,
    model::channel::Message,
    prelude::*,
};
use std::{collections::HashSet, env, sync::Arc};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
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
#[commands(ping, help, account, rating, gdpr)]
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
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id = env::var("APPLICATION_ID")
        .expect("Expected to find the APPLICATION_ID environment variable")
        .parse()
        .expect("Expected the APPLICATION_ID environment variable to be an integer");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| {
            c.owners(owners)
                .with_whitespace(true)
                .prefix("") // disable default ~ prefix
                .prefixes(vec!["ohnomy", "oh no my"])
                .case_insensitivity(true)
                .on_mention(Some(bot_id))
                .ignore_bots(true)
        })
        .unrecognised_command(unknown_command)
        .group(&GENERAL_GROUP);

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler {})
        .application_id(application_id)
        .intents(
            GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::GUILDS,
        )
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
        shard_manager.lock().await.shutdown_all().await;
    });

    info!("Starting bot");

    match client.start().await {
        Ok(_) => info!("Bot shutting down"),
        Err(why) => error!("Bot returned an error: {:?}", why),
    }
}
