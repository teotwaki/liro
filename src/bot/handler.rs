use super::run::{PoolContainer, RoleManagerContainer};
use crate::{
    bot::{
        commands::{
            account::{link, unlink},
            rating_update::update_ratings,
            Response as CommandResponse,
        },
        rating_range::RatingRange,
    },
    models,
};
use serenity::{
    async_trait,
    model::{
        interactions::application_command::{
            ApplicationCommand, ApplicationCommandInteractionDataOptionValue,
            ApplicationCommandOptionType,
        },
        {gateway::Ready, guild::Guild, prelude::*},
    },
    prelude::*,
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_create(&self, ctx: Context, guild: Guild) {
        trace!("Handler::guild_create() called");
        let data = ctx.data.read().await;

        let guild_id = *guild.id.as_u64();
        {
            let pool = data.get::<PoolContainer>().unwrap().clone();
            match models::Guild::new(&pool, guild_id, &guild.name).await {
                Ok(guild) => info!("Joining new {}", guild),
                Err(e) => {
                    error!("Unable to save guild: {}", e);
                    return;
                }
            }
        }

        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();
        for (role_id, role) in &guild.roles {
            let role_id = *role_id.as_u64();
            if let Ok(rr) = role.name.parse::<RatingRange>() {
                info!(
                    "Adding new role {} (role_id={}) to guild {} (guild_id={})",
                    role.name, role_id, guild.name, guild_id
                );
                role_manager.add_rating_range(guild_id, role_id, rr);
            }
        }
    }

    async fn guild_delete(&self, ctx: Context, guild: GuildUnavailable) {
        trace!("Handler::guild_delete() called");
        let guild_id = *guild.id.as_u64();
        let data = ctx.data.read().await;
        let pool = data.get::<PoolContainer>().unwrap().clone();

        match models::Guild::find(&pool, guild_id).await {
            Ok(Some(guild)) => {
                info!("Deleting {}", guild);
                if let Err(e) = guild.delete(&pool).await {
                    error!("Unable to delete guild_id={}: {}", guild_id, e);
                    return;
                }
            }
            Ok(None) => info!(
                "Ignoring request to delete non-existent guild_id={}",
                guild_id
            ),
            Err(e) => {
                error!("Unable to remove guild_id={}: {}", guild_id, e);
                return;
            }
        }

        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();
        role_manager.delete_guild(guild_id);
    }

    async fn guild_role_create(&self, ctx: Context, guild_id: GuildId, role: Role) {
        trace!("Handler::guild_role_create() called");
        info!(
            "Adding role {} (role_id={}) to guild_id={}",
            role.name, role.id, guild_id
        );
        let data = ctx.data.read().await;
        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();

        if let Ok(rr) = role.name.parse::<RatingRange>() {
            role_manager.add_rating_range(*guild_id.as_u64(), *role.id.as_u64(), rr);
        }
    }

    async fn guild_role_update(&self, ctx: Context, guild_id: GuildId, role: Role) {
        trace!("Handler::guild_role_update() called");
        let guild_id = *guild_id.as_u64();
        let role_id = *role.id.as_u64();

        let data = ctx.data.read().await;
        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();

        role_manager.remove_role(guild_id, role_id);

        if let Ok(rr) = role.name.parse::<RatingRange>() {
            info!(
                "Updating role {} (role_id={}) in guild_id={}",
                role.name, role_id, guild_id
            );
            role_manager.add_rating_range(guild_id, role_id, rr);
        }
    }

    async fn guild_role_delete(&self, ctx: Context, guild_id: GuildId, role_id: RoleId) {
        trace!("Handler::guild_role_delete() called");
        info!("Removing role_id={} from guild_id={}", role_id, guild_id);
        let data = ctx.data.read().await;
        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();

        role_manager.remove_role(*guild_id.as_u64(), *role_id.as_u64());
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, ctx: Context, ready: Ready) {
        trace!("Handler::ready() called");
        info!("{} is now online", ready.user.tag());

        let commands = ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command.name("rating").description(
                        "Retrieves your updated lichess ratings and gives you Discord roles",
                    )
                })
                .create_application_command(|command| {
                    command.name("link").description(
                        "Connects your lichess.org or chess.com account with Liro. Needed to update ratings.",
                    ).create_option(|option| {
                        option
                            .name("website")
                            .description("The website you would like to link against")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                            .add_string_choice("lichess.org", "lichess")
                            .add_string_choice("chess.com", "chesscom")
                    })
                })
                .create_application_command(|command| {
                    command.name("unlink").description(
                        "Deletes all your information from the bot and removes your Discord roles.",
                    )
                })
        })
        .await;

        match commands {
            Ok(commands) => debug!(
                "Installed the following global application commands: {:?}",
                commands
            ),
            Err(why) => error!("{}", why),
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        trace!("Handler::interaction_create()");
        if let Interaction::ApplicationCommand(command) = interaction {
            let guild_id = match command.guild_id {
                Some(guild_id) => *guild_id.as_u64(),
                None => {
                    error!("Failed to handle interaction: missing guild_id in command");
                    return;
                }
            };

            let discord_id = *command.user.id.as_u64();
            info!(
                "Handling application command '/{}' for discord_id={} in guild_id={}",
                command.data.name, discord_id, guild_id
            );
            let command_response = match command.data.name.as_str() {
                "rating" => update_ratings(&ctx, guild_id, discord_id).await,
                "link" => {
                    let option = command
                        .data
                        .options
                        .get(0)
                        .unwrap()
                        .resolved
                        .as_ref()
                        .unwrap();

                    if let ApplicationCommandInteractionDataOptionValue::String(website) = option {
                        link(&ctx, guild_id, discord_id, website.to_string()).await
                    } else {
                        return;
                    }
                }
                "unlink" => unlink(&ctx, guild_id, discord_id).await,
                _ => unreachable!(),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response.interaction_response_data(|message| match command_response {
                        Ok(CommandResponse::Embed(e)) => message.add_embed(e),
                        Ok(CommandResponse::PrivateEmbed(e)) => message
                            .add_embed(e)
                            .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL),
                        Ok(CommandResponse::Sentence(s)) => message.content(s),
                        Ok(CommandResponse::PrivateSentence(s)) => message
                            .content(s)
                            .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL),
                        Err(why) => {
                            error!("Error handling command: {}", why);
                            message.content("Internal bot error. @teotwaki, I'm scared.")
                        }
                    })
                })
                .await
            {
                error!("Cannot respond to slash command: {}", why);
            }
        }
    }
}
