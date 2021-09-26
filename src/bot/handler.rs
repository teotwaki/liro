use super::run::{PoolContainer, RoleManagerContainer};
use crate::{bot::rating_range::RatingRange, models};
use serenity::{
    async_trait,
    model::{gateway::Ready, guild::Guild, prelude::*},
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
    async fn ready(&self, _: Context, ready: Ready) {
        trace!("Handler::ready() called");
        info!("{} is now online", ready.user.tag());
    }
}
