use super::{Data, commands::Result};
use crate::models;
use poise::serenity_prelude::FullEvent;

pub async fn handle(event: &FullEvent, data: &Data) -> Result<()> {
    match event {
        FullEvent::Ready { data_about_bot } => {
            info!("{} is now online", data_about_bot.user.tag());
        }
        FullEvent::GuildCreate { guild, .. } => {
            data.roles.replace_guild_roles(
                guild.id,
                guild
                    .roles
                    .values()
                    .filter_map(|role| role.name.parse().ok().map(|range| (role.id, range))),
            );
            let guild = models::Guild::new(&data.pool, guild.id, &guild.name).await?;
            info!("Loaded {guild}");
        }
        FullEvent::GuildDelete { incomplete, .. } => {
            let guild_id = incomplete.id;
            if incomplete.unavailable {
                debug!("Guild {guild_id} is temporarily unavailable; preserving its state");
                return Ok(());
            }
            data.roles.delete_guild(guild_id);
            if let Some(guild) = models::Guild::find(&data.pool, guild_id).await? {
                guild.delete(&data.pool).await?;
                info!("Deleted {guild}");
            }
        }
        FullEvent::GuildRoleCreate { new: role } | FullEvent::GuildRoleUpdate { new: role, .. } => {
            data.roles.set_role_name(role.guild_id, role.id, &role.name);
        }
        FullEvent::GuildRoleDelete {
            guild_id,
            removed_role_id,
            ..
        } => {
            data.roles.remove_role(*guild_id, *removed_role_id);
        }
        _ => {}
    }
    Ok(())
}
