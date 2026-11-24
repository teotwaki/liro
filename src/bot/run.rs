use super::{
    Data,
    commands::{self, Error},
    handler,
    role_manager::RoleManager,
};
use crate::{db::Pool, lichess};
use poise::serenity_prelude::{
    Client, CreateAllowedMentions, CreateCommand, CreateMessage, Error as SerenityError,
    GatewayIntents, GuildId, Http,
};
use std::env;

const TEXT_PREFIX_PATTERN: &str = r"(?i)\A(?:ohnomy|oh no my)";

fn registration_guild(
    value: Result<String, env::VarError>,
) -> Result<Option<GuildId>, &'static str> {
    match value {
        Ok(value) => value
            .parse::<GuildId>()
            .map(Some)
            .map_err(|_| "DISCORD_TEST_GUILD_ID must be a nonzero Discord server ID"),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err("DISCORD_TEST_GUILD_ID is not valid Unicode"),
    }
}

fn guild_command_payload(
    commands: Vec<CreateCommand>,
) -> serde_json::Result<Vec<serde_json::Value>> {
    commands
        .into_iter()
        .map(|command| {
            let mut payload = serde_json::to_value(command)?;
            // Poise's guild_only generates a context for GLOBAL commands. Discord's
            // guild endpoint doesn't take context fields; runtime checks stay intact.
            if let Some(fields) = payload.as_object_mut() {
                for field in ["contexts", "integration_types", "dm_permission"] {
                    fields.remove(field);
                }
            }
            Ok(payload)
        })
        .collect()
}

async fn register_commands(
    http: &Http,
    commands: &[poise::Command<Data, Error>],
    guild: Option<GuildId>,
) -> commands::Result<()> {
    if let Some(guild_id) = guild {
        let builders = poise::builtins::create_application_commands(commands);
        let payload = guild_command_payload(builders).map_err(SerenityError::from)?;
        http.create_guild_commands(guild_id, &payload).await?;
        info!("Registered slash commands in test guild {guild_id}");
    } else {
        poise::builtins::register_globally(http, commands).await?;
        info!("Registered global slash commands");
    }
    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    let result = match error {
        poise::FrameworkError::UnknownCommand {
            msg,
            content_start,
            framework,
            trigger: poise::MessageDispatchTrigger::MessageCreate,
            ..
        } => {
            let name = msg.content[usize::from(content_start)..]
                .split_whitespace()
                .next();
            if let Some(name) = name {
                let message = format!(
                    "Could not understand command `{}`. Please see `ohnomy help` for more information",
                    name.replace('`', "")
                );
                msg.channel_id
                    .send_message(
                        &framework.serenity_context.http,
                        CreateMessage::new()
                            .content(message)
                            .allowed_mentions(CreateAllowedMentions::new()),
                    )
                    .await
                    .map(|_| ())
            } else {
                Ok(())
            }
        }
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!("Error handling {}: {error}", ctx.command().name);
            ctx.say("Internal bot error. Please try again later.")
                .await
                .map(|_| ())
        }
        error => poise::builtins::on_error(error).await,
    };
    if let Err(error) = result {
        error!("Unable to handle Discord command error: {error}");
    }
}

pub async fn run(pool: &Pool, lichess: &lichess::Client) {
    let guild = match registration_guild(env::var("DISCORD_TEST_GUILD_ID")) {
        Ok(guild) => guild,
        Err(error) => {
            error!("{error}; refusing to register commands");
            return;
        }
    };
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id = env::var("APPLICATION_ID")
        .expect("Expected to find the APPLICATION_ID environment variable")
        .parse()
        .expect("Expected APPLICATION_ID to be a nonzero Discord application ID");
    let data = Data {
        pool: pool.clone(),
        lichess: lichess.clone(),
        roles: RoleManager::new(),
    };
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::registry(),
            initialize_owners: false,
            allowed_mentions: Some(CreateAllowedMentions::new()),
            prefix_options: poise::PrefixFrameworkOptions {
                additional_prefixes: vec![poise::Prefix::Regex(
                    regex::Regex::new(TEXT_PREFIX_PATTERN).expect("valid prefix pattern"),
                )],
                case_insensitive_commands: true,
                mention_as_prefix: true,
                ignore_bots: true,
                ..Default::default()
            },
            event_handler: |framework, event| Box::pin(handler::handle(event, framework.user_data)),
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .setup(move |ctx, _, framework| {
            Box::pin(async move {
                if let Err(error) =
                    register_commands(&ctx.http, &framework.options().commands, guild).await
                {
                    match guild {
                        Some(id) => {
                            error!("Unable to register slash commands in test guild {id}: {error}")
                        }
                        None => error!("Unable to register global slash commands: {error}"),
                    }
                }
                // Poise's setup must initialize data even if Discord rejects registration.
                Ok(data)
            })
        })
        .build();
    let intents = GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILDS;

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    let shard_manager = client.shard_manager.clone();
    let shutdown = tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.shutdown_all().await;
    });

    info!("Starting bot");
    match client.start().await {
        Ok(_) => info!("Bot shutting down"),
        Err(why) => error!("Bot returned an error: {why:?}"),
    }
    shutdown.abort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_missing_test_guild_means_global_registration() {
        assert_eq!(registration_guild(Err(env::VarError::NotPresent)), Ok(None));
        assert_eq!(
            registration_guild(Ok("42".into())),
            Ok(Some(GuildId::new(42)))
        );
        for invalid in ["", "0", "invalid", "18446744073709551616"] {
            assert!(registration_guild(Ok(invalid.into())).is_err());
        }
        assert!(registration_guild(Err(env::VarError::NotUnicode("invalid".into()))).is_err());
    }

    #[test]
    fn legacy_prefixes_reach_the_same_commands() {
        let pattern = regex::Regex::new(TEXT_PREFIX_PATTERN).unwrap();
        let commands = commands::registry();
        for (input, expected) in [
            ("oh no my AcCoUnT", "account"),
            ("OHNOMY \tGdpr", "gdpr"),
            ("ohnomyrating", "rating"),
            ("ohnomy help", "help"),
        ] {
            let prefix = pattern.find(input).unwrap();
            let mut tree = Vec::new();
            poise::find_command(
                &commands,
                input[prefix.end()..].trim_start(),
                true,
                &mut tree,
            )
            .unwrap();
            assert_eq!(tree[0].name, expected);
            assert!(tree[0].prefix_action.is_some());
        }
        assert!(!pattern.is_match("normal oh no my chat"));
        assert!(!pattern.is_match("~help"));
    }

    #[test]
    fn slash_payload_only_contains_the_three_guild_commands() {
        let commands = commands::registry();
        let builders = poise::builtins::create_application_commands(&commands);
        let global = serde_json::to_value(&builders).unwrap();
        let guild = guild_command_payload(builders).unwrap();
        assert_eq!(guild.len(), 3);
        for (i, name) in ["rating", "link", "unlink"].into_iter().enumerate() {
            assert_eq!(guild[i]["name"], name);
            assert_eq!(global[i]["contexts"], serde_json::json!([0]));
            assert!(guild[i].get("contexts").is_none());
            assert!(guild[i].get("integration_types").is_none());
            assert!(guild[i].get("dm_permission").is_none());
        }
    }
}
