use super::rating_range::RatingRange;
use regex::Regex;
use serenity::prelude::Mutex;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
pub struct GuildRoleManager {
    guild_rating_ranges: HashMap<u64, Vec<RatingRange>>,
    under_re: Regex,
    over_re: Regex,
    range_re: Regex,
}

impl GuildRoleManager {
    pub fn new() -> Arc<Mutex<GuildRoleManager>> {
        trace!("GuildRoleManager::new() called");
        let role_manager = GuildRoleManager {
            guild_rating_ranges: Default::default(),
            under_re: Regex::new(r"U(?P<max>\d{3,4})").unwrap(),
            over_re: Regex::new(r"(?P<min>\d{3,4})+").unwrap(),
            range_re: Regex::new(r"(?P<min>\d{3,4})-(?P<max>\d{3,4})").unwrap(),
        };
        Arc::new(Mutex::new(role_manager))
    }

    pub fn parse_rating_range(&self, role_id: u64, name: &str) -> Option<RatingRange> {
        trace!("GuildRoleManager::parse_rating_range() called");
        if let Some(captures) = self.range_re.captures(name) {
            let min = captures.name("min")?.as_str().parse().ok();
            let max = captures.name("max")?.as_str().parse().ok();
            Some(RatingRange::new(role_id, min, max))
        } else if let Some(captures) = self.under_re.captures(name) {
            let max = captures
                .name("max")?
                .as_str()
                .parse::<i16>()
                .ok()
                .and_then(|max| Some(max - 1));
            Some(RatingRange::new(role_id, None, max))
        } else if let Some(captures) = self.over_re.captures(name) {
            let min = captures.name("min")?.as_str().parse().ok();
            Some(RatingRange::new(role_id, min, None))
        } else {
            None
        }
    }

    /// Initializes a new guild in the manager
    ///
    /// This overwrites previously-existing guilds. If a guild with the same ID already existed, it
    /// will be overwritten.
    pub fn add_guild(&mut self, guild_id: u64) {
        trace!("GuildRoleManager::add_guild() called");
        self.guild_rating_ranges
            .insert(guild_id, Default::default());
    }

    /// Adds a new rating range role for the specific `guild_id`
    ///
    /// If the `guild_id` does not exist in the role manager, nothing will happen. This function
    /// does not panic or throw an error.
    pub fn add_rating_range(&mut self, guild_id: u64, rating: RatingRange) {
        trace!("GuildRoleManager::add_rating_range() called");
        self.guild_rating_ranges
            .get_mut(&guild_id)
            .map(|grr| grr.push(rating));
    }

    pub fn remove_role(&mut self, guild_id: u64, role_id: u64) {
        trace!("GuildRoleManager::remove_role() called");
        self.guild_rating_ranges.get_mut(&guild_id).map(|ranges| {
            ranges
                .iter()
                .position(|r| r.role_id() == role_id)
                .map(|position| ranges.remove(position));
        });
    }

    pub fn find_rating_range_role(&self, guild_id: u64, rating: i16) -> Option<u64> {
        trace!("GuildRoleManager::find_rating_range_role() called");
        let ranges = self.guild_rating_ranges.get(&guild_id)?;
        ranges
            .iter()
            .find(|r| r.is_match(rating))
            .map(|r| r.role_id())
    }

    pub fn other_rating_range_roles(&self, guild_id: u64, role_id: u64) -> Vec<u64> {
        trace!("GuildRoleManager::other_rating_range_roles() called");
        match self.guild_rating_ranges.get(&guild_id) {
            Some(ranges) => ranges
                .iter()
                .filter(|r| r.role_id() != role_id)
                .map(|r| r.role_id())
                .collect(),
            None => Default::default(),
        }
    }
}
