use super::commands::account::*;
use super::commands::meta::*;
use crate::{bot::Handler, db::Pool};
use regex::Regex;
use serenity::{
    client::bridge::gateway::{GatewayIntents, ShardManager},
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    env, fmt,
    sync::Arc,
};
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

#[derive(Debug, Clone, Copy)]
pub struct RatingRange {
    role_id: u64,
    min: Option<i16>,
    max: Option<i16>,
}

impl RatingRange {
    pub fn new(role_id: u64, min: Option<i16>, max: Option<i16>) -> RatingRange {
        let rr = RatingRange { role_id, min, max };
        debug!("Creating new {}", rr);
        rr
    }

    pub fn is_match(&self, rating: i16) -> bool {
        match (self.min, self.max) {
            (Some(min), Some(max)) => rating > min && rating < max,
            (Some(min), _) => rating > min,
            (_, Some(max)) => rating < max,
            _ => false,
        }
    }
}

impl fmt::Display for RatingRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.min, self.max) {
            (Some(min), Some(max)) => write!(
                f,
                "RatingRange<role_id={} min={} max={}",
                self.role_id, min, max
            ),
            (Some(min), None) => write!(
                f,
                "RatingRange<role_id={} min={} max=None",
                self.role_id, min
            ),
            (None, Some(max)) => write!(
                f,
                "RatingRange<role_id={} min=None max={}",
                self.role_id, max
            ),
            (None, None) => write!(f, "RatingRange<role_id={} min=None max=None", self.role_id),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuildRoleManager {
    ranges: HashMap<u64, Vec<RatingRange>>,
    under_re: Regex,
    over_re: Regex,
    range_re: Regex,
}

impl GuildRoleManager {
    pub fn new() -> Arc<Mutex<GuildRoleManager>> {
        let role_manager = GuildRoleManager {
            ranges: Default::default(),
            under_re: Regex::new(r"U(?P<max>\d{3,4})").unwrap(),
            over_re: Regex::new(r"(?P<min>\d{3,4})+").unwrap(),
            range_re: Regex::new(r"(?P<min>\d{3,4})-(?P<max>\d{3,4})").unwrap(),
        };
        Arc::new(Mutex::new(role_manager))
    }

    pub fn parse_rating_range(&self, role_id: u64, name: &str) -> Option<RatingRange> {
        if let Some(captures) = self.range_re.captures(name) {
            let min = captures.name("min")?.as_str().parse().ok();
            let max = captures.name("max")?.as_str().parse().ok();
            Some(RatingRange::new(role_id, min, max))
        } else if let Some(captures) = self.under_re.captures(name) {
            let max = captures.name("max")?.as_str().parse().ok();
            Some(RatingRange::new(role_id, None, max))
        } else if let Some(captures) = self.over_re.captures(name) {
            let min = captures.name("min")?.as_str().parse().ok();
            Some(RatingRange::new(role_id, min, None))
        } else {
            None
        }
    }

    pub fn add_role(&mut self, guild_id: u64, rating: RatingRange) {
        if !self.ranges.contains_key(&guild_id) {
            self.ranges.insert(guild_id, Default::default());
        }
        let guild_roles = self.ranges.get_mut(&guild_id).unwrap();

        guild_roles.push(rating);
    }

    pub fn find_rating_role(&self, guild_id: u64, rating: i16) -> Option<u64> {
        let ranges = self.ranges.get(&guild_id)?;
        ranges
            .iter()
            .find(|r| r.is_match(rating))
            .map(|r| r.role_id)
    }

    pub fn other_roles(&self, guild_id: u64, role_id: u64) -> Vec<u64> {
        match self.ranges.get(&guild_id) {
            Some(ranges) => ranges
                .iter()
                .filter(|r| r.role_id != role_id)
                .map(|r| r.role_id)
                .collect(),
            None => Default::default(),
        }
    }
}

pub struct GuildRoleManagerContainer;

impl TypeMapKey for GuildRoleManagerContainer {
    type Value = Arc<Mutex<GuildRoleManager>>;
}

#[group]
#[commands(ping, account, rating)]
struct General;

pub async fn run(pool: &Pool) {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("ohnomy "))
        .group(&GENERAL_GROUP);

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler {})
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
        data.insert::<GuildRoleManagerContainer>(GuildRoleManager::new());
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
