use super::run::{PoolContainer, RoleManagerContainer};
use crate::{
    bot::{
        commands::{
            Response as CommandResponse,
            account::{link, unlink},
            rating_update::update_ratings,
        },
        rating_range::RatingRange,
    },
    models,
};
use serenity::{
    all::{CreateCommand, CreateInteractionResponse},
    async_trait,
    model::{application::Command, gateway::Ready, guild::Guild, prelude::*},
    prelude::*,
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        trace!("Handler::guild_create() called");
        let data = ctx.data.read().await;

        {
            let pool = data.get::<PoolContainer>().unwrap().clone();
            match models::Guild::new(&pool, guild.id, &guild.name).await {
                Ok(guild) => info!("Joining new {}", guild),
                Err(e) => {
                    error!("Unable to save guild: {}", e);
                    return;
                }
            }
        }

        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();
        for (role_id, role) in &guild.roles {
            if let Ok(rr) = role.name.parse::<RatingRange>() {
                info!(
                    "Adding new role {} (role_id={}) to guild {} (guild_id={})",
                    role.name, role_id, guild.name, guild.id
                );
                role_manager.add_rating_range(guild.id, *role_id, rr);
            }
        }
    }

    async fn guild_delete(&self, ctx: Context, guild: UnavailableGuild, _full: Option<Guild>) {
        trace!("Handler::guild_delete() called");
        let guild_id = guild.id;
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

    async fn guild_role_create(&self, ctx: Context, role: Role) {
        trace!("Handler::guild_role_create() called");
        info!(
            "Adding role {} (role_id={}) to guild_id={}",
            role.name, role.id, role.guild_id
        );
        let data = ctx.data.read().await;
        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();

        if let Ok(rr) = role.name.parse::<RatingRange>() {
            role_manager.add_rating_range(role.guild_id, role.id, rr);
        }
    }

    async fn guild_role_update(
        &self,
        ctx: Context,
        _old_data_if_available: Option<Role>,
        role: Role,
    ) {
        trace!("Handler::guild_role_update() called");
        let data = ctx.data.read().await;
        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();

        role_manager.remove_role(role.guild_id, role.id);

        if let Ok(rr) = role.name.parse::<RatingRange>() {
            info!(
                "Updating role {} (role_id={}) in guild_id={}",
                role.name, role.id, role.guild_id
            );
            role_manager.add_rating_range(role.guild_id, role.id, rr);
        }
    }

    async fn guild_role_delete(
        &self,
        ctx: Context,
        guild_id: GuildId,
        role_id: RoleId,
        _role_data_if_available: Option<Role>,
    ) {
        trace!("Handler::guild_role_delete() called");
        info!("Removing role_id={} from guild_id={}", role_id, guild_id);
        let data = ctx.data.read().await;
        let mut role_manager = data.get::<RoleManagerContainer>().unwrap().clone();

        role_manager.remove_role(guild_id, role_id);
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

        let commands = Command::set_global_commands(
            &ctx.http,
            vec![
                CreateCommand::new("rating").description(
                    "Retrieves your updated lichess ratings and gives you Discord roles",
                ),
                CreateCommand::new("link").description(
                    "Connects your lichess.org account with Liro. Needed to update ratings.",
                ),
                CreateCommand::new("unlink").description(
                    "Deletes all your information from the bot and removes your Discord roles.",
                ),
            ],
        )
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
        if let Interaction::Command(command) = interaction {
            let guild_id = match command.guild_id {
                Some(guild_id) => guild_id,
                None => {
                    error!("Failed to handle interaction: missing guild_id in command");
                    return;
                }
            };

            let discord_id = command.user.id;
            info!(
                "Handling application command '/{}' for discord_id={} in guild_id={}",
                command.data.name, discord_id, guild_id
            );
            let command_response = match command.data.name.as_str() {
                "rating" => update_ratings(&ctx, guild_id, discord_id).await,
                "link" => link(&ctx, guild_id, discord_id).await,
                "unlink" => unlink(&ctx, guild_id, discord_id).await,
                _ => unreachable!(),
            };

            let result = command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(match command_response {
                        Ok(CommandResponse::Embed(e)) => {
                            serenity::builder::CreateInteractionResponseMessage::new().add_embed(e)
                        }
                        Ok(CommandResponse::PrivateEmbed(e)) => {
                            serenity::builder::CreateInteractionResponseMessage::new()
                                .add_embed(e)
                                .flags(InteractionResponseFlags::EPHEMERAL)
                        }
                        Ok(CommandResponse::Sentence(s)) => {
                            serenity::builder::CreateInteractionResponseMessage::new().content(s)
                        }
                        Ok(CommandResponse::PrivateSentence(s)) => {
                            serenity::builder::CreateInteractionResponseMessage::new()
                                .content(s)
                                .flags(InteractionResponseFlags::EPHEMERAL)
                        }
                        Err(ref why) => {
                            error!("Error handling command: {}", why);
                            serenity::builder::CreateInteractionResponseMessage::new()
                                .content("Internal bot error. @teotwaki, I'm scared.")
                        }
                    }),
                )
                .await;

            if let Err(why) = result {
                error!("Cannot respond to slash command: {}", why);
            }
        }
    }
}
