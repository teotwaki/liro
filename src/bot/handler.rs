use super::run::GuildRoleManagerContainer;
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
        info!("Joining new guild {}", guild.name);
        let guild_id = *guild.id.as_u64();
        let data = ctx.data.read().await;
        let role_manager = data.get::<GuildRoleManagerContainer>().unwrap();
        let mut dg = role_manager.lock().await;

        dg.add_guild(guild_id);

        for (role_id, role) in &guild.roles {
            dg.parse_rating_range(*role_id.as_u64(), &role.name)
                .map(|rr| dg.add_rating_range(guild_id, rr));
        }
    }

    async fn guild_role_create(&self, ctx: Context, guild_id: GuildId, role: Role) {
        trace!("Handler::guild_role_create() called");
        info!(
            "Adding role {} (role_id={}) in guild_id{}",
            role.name, role.id, guild_id
        );
        let data = ctx.data.read().await;
        let role_manager = data.get::<GuildRoleManagerContainer>().unwrap();
        let mut dg = role_manager.lock().await;

        dg.parse_rating_range(*role.id.as_u64(), &role.name)
            .map(|rr| dg.add_rating_range(*guild_id.as_u64(), rr));
    }

    async fn guild_role_delete(&self, ctx: Context, guild_id: GuildId, role_id: RoleId) {
        trace!("Handler::guild_role_delete() called");
        debug!("Removing role_id={} from guild_id={}", role_id, guild_id);
        let data = ctx.data.read().await;
        let role_manager = data.get::<GuildRoleManagerContainer>().unwrap();
        let mut dg = role_manager.lock().await;

        dg.remove_role(*guild_id.as_u64(), *role_id.as_u64());
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
