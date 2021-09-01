use super::Command;
use crate::{
    db, lichess,
    models::{Challenge, User},
};
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, guild::Guild},
    prelude::*,
};

enum Response {
    Channel(String),
    WhisperAndChannel(String, String),
}

pub struct Handler {
    pool: db::Pool,
}

impl Handler {
    pub async fn new() -> Self {
        Self {
            pool: db::connect().await.unwrap(),
        }
    }

    async fn handle<'a>(&self, cmd: &Command<'a>) -> Option<Response> {
        match cmd {
            Command::Rating(discord_id) => {
                debug!(
                    "Handling rating command for user with discord_id={}",
                    discord_id
                );
                match User::find(&self.pool, *discord_id).await {
                    Ok(Some(mut user)) => {
                        let rating = lichess::api::fetch_user_rating(&user.lichess_username()).await;
                        match rating {
                            Ok(rating) => user.update_rating(&self.pool, rating)
                                .await
                                .ok()
                                .map(|_| Response::Channel(format!("Your rating is now {}", rating))),
                            Err(why) => {
                                error!("Couldn't fetch updated rating: {}", why);
                                None
                            }
                        }
                    }
                    Ok(None) => {
                        Some(Response::Channel("Couldn't find a lichess user associated with your account. Please use the `account` command first.".to_string()))
                    }
                    Err(why) => {
                        error!("Unable to query database: {}", why);
                        None
                    }
                }
            }
            Command::ConnectAccount(discord_id) => {
                match Challenge::new(&self.pool, *discord_id).await {
                    Ok(challenge) => {
                        let whisper = format!(
                            "Please connect your account using the following link: {}",
                            challenge.link()
                        );
                        let channel = "Please check your DMs :)".to_string();

                        Some(Response::WhisperAndChannel(whisper, channel))
                    }
                    Err(why) => {
                        error!("Unable to create new challenge: {:?}", why);
                        None
                    }
                }
            }
            Command::Unknown(_) => Some(Response::Channel(
                "Not sure what you meant with that".to_string(),
            )),
            Command::Ignore => None,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        let command = Command::parse(&msg).await;
        let response = self.handle(&command).await;

        match response {
            Some(Response::Channel(response)) => {
                if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                    error!("Error sending message: {:?}", why);
                }
            }
            Some(Response::WhisperAndChannel(whisper, response)) => {
                if let Err(why) = msg
                    .author
                    .dm(&ctx, |m| {
                        m.content(whisper);
                        m
                    })
                    .await
                {
                    error!("Error sending direct message: {:?}", why);
                }

                if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                    error!("Error sending message: {:?}", why);
                }
            }
            None => {}
        }
    }

    async fn guild_create(&self, _ctx: Context, guild: Guild) {
        debug!("{:?}", guild);
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is now online", ready.user.tag());
    }
}
