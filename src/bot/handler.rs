use super::run::GuildRoleManagerContainer;
use serenity::{
    async_trait,
    model::{gateway::Ready, guild::Guild},
    prelude::*,
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_create(&self, ctx: Context, guild: Guild) {
        let data = ctx.data.read().await;
        let role_manager = data.get::<GuildRoleManagerContainer>().unwrap();
        let mut role_manager_dg = role_manager.lock().await;

        for (role_id, role) in &guild.roles {
            role_manager_dg
                .parse_rating_range(*role_id.as_u64(), &role.name)
                .map(|role_rating_range| {
                    role_manager_dg.add_role(*guild.id.as_u64(), role_rating_range)
                });
        }
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
