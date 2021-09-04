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
        let role_manager = GuildRoleManager {
            guild_rating_ranges: Default::default(),
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

    pub fn add_rating_range(&mut self, guild_id: u64, rating: RatingRange) {
        if !self.guild_rating_ranges.contains_key(&guild_id) {
            self.guild_rating_ranges
                .insert(guild_id, Default::default());
        }
        let guild_roles = self.guild_rating_ranges.get_mut(&guild_id).unwrap();

        guild_roles.push(rating);
    }

    pub fn remove_role(&mut self, guild_id: u64, role_id: u64) {
        self.guild_rating_ranges.get_mut(&guild_id).map(|ranges| {
            ranges
                .iter()
                .position(|r| r.role_id() == role_id)
                .map(|position| ranges.remove(position));
        });
    }

    pub fn find_rating_range_role(&self, guild_id: u64, rating: i16) -> Option<u64> {
        let ranges = self.guild_rating_ranges.get(&guild_id)?;
        ranges
            .iter()
            .find(|r| r.is_match(rating))
            .map(|r| r.role_id())
    }

    pub fn other_rating_range_roles(&self, guild_id: u64, role_id: u64) -> Vec<u64> {
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
