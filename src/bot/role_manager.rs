use super::rating_range::RatingRange;
use regex::Regex;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct GuildRoleManager {
    guild_rating_ranges: Arc<Mutex<HashMap<u64, Vec<RatingRange>>>>,
    under_re: Regex,
    over_re: Regex,
    range_re: Regex,
}

impl GuildRoleManager {
    pub fn new() -> GuildRoleManager {
        trace!("GuildRoleManager::new() called");
        GuildRoleManager {
            guild_rating_ranges: Default::default(),
            under_re: Regex::new(r"^(U|u)(?P<max>\d{3,4})$").unwrap(),
            over_re: Regex::new(r"^(?P<min>\d{3,4})\+$").unwrap(),
            range_re: Regex::new(r"^(?P<min>\d{3,4})-(?P<max>\d{3,4})$").unwrap(),
        }
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
                .map(|max| max - 1);
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
            .lock()
            .unwrap()
            .insert(guild_id, Default::default());
    }

    /// Adds a new rating range role for the specific `guild_id`
    ///
    /// If the `guild_id` does not exist in the role manager, nothing will happen. This function
    /// does not panic or throw an error.
    pub fn add_rating_range(&mut self, guild_id: u64, rating: RatingRange) {
        trace!("GuildRoleManager::add_rating_range() called");
        if let Some(grr) = self.guild_rating_ranges.lock().unwrap().get_mut(&guild_id) {
            grr.push(rating);
        }
    }

    pub fn remove_role(&mut self, guild_id: u64, role_id: u64) {
        trace!("GuildRoleManager::remove_role() called");
        if let Some(ranges) = self.guild_rating_ranges.lock().unwrap().get_mut(&guild_id) {
            ranges
                .iter()
                .position(|r| r.role_id() == role_id)
                .map(|position| ranges.remove(position));
        }
    }

    pub fn find_rating_range_role(&self, guild_id: u64, rating: i16) -> Option<u64> {
        trace!("GuildRoleManager::find_rating_range_role() called");
        self.guild_rating_ranges
            .lock()
            .unwrap()
            .get(&guild_id)?
            .iter()
            .find(|r| r.is_match(rating))
            .map(|r| r.role_id())
    }

    pub fn other_rating_range_roles(&self, guild_id: u64, role_id: u64) -> Vec<u64> {
        trace!("GuildRoleManager::other_rating_range_roles() called");
        match self.guild_rating_ranges.lock().unwrap().get(&guild_id) {
            Some(ranges) => ranges
                .iter()
                .filter(|r| r.role_id() != role_id)
                .map(|r| r.role_id())
                .collect(),
            None => Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rating_range_handles_exclusively_under() {
        let grm = GuildRoleManager::new();
        let parsed = grm.parse_rating_range(0, "U1000").unwrap();
        let rr = RatingRange::new(0, None, Some(999));

        assert_eq!(parsed, rr);
    }

    #[test]
    fn parse_rating_range_handles_upper_and_lowercase() {
        let grm = GuildRoleManager::new();
        let upper = grm.parse_rating_range(0, "U1000");
        let lower = grm.parse_rating_range(0, "u1000");

        assert_eq!(upper, lower);
    }

    #[test]
    fn parse_rating_range_handles_exclusively_over() {
        let grm = GuildRoleManager::new();
        let parsed = grm.parse_rating_range(0, "2200+").unwrap();
        let rr = RatingRange::new(0, Some(2200), None);

        assert_eq!(parsed, rr);
    }

    #[test]
    fn parse_rating_range_handles_in_between() {
        let grm = GuildRoleManager::new();
        let parsed = grm.parse_rating_range(0, "1000-1099").unwrap();
        let rr = RatingRange::new(0, Some(1000), Some(1099));

        assert_eq!(parsed, rr);
    }

    #[test]
    fn parse_rating_range_ignores_random_strings() {
        let grm = GuildRoleManager::new();

        assert!(grm.parse_rating_range(0, "foo").is_none()); // random
        assert!(grm.parse_rating_range(0, "2000++").is_none()); // invalid suffix
        assert!(grm.parse_rating_range(0, "uu2000").is_none()); // invalid prefix
        assert!(grm.parse_rating_range(0, "10-2000").is_none()); // value too small (10)
        assert!(grm.parse_rating_range(0, "100-20000").is_none()); // value too big (20000)
    }

    #[test]
    fn find_rating_range_role_can_be_called_on_an_empty_manager() {
        let grm = GuildRoleManager::new();

        assert!(grm.find_rating_range_role(0, 0).is_none());
    }

    #[test]
    fn adding_roles_to_a_nonexistent_guild_does_nothing() {
        let mut grm = GuildRoleManager::new();
        grm.add_rating_range(0, RatingRange::new(0, Some(10), Some(20)));

        assert!(grm.find_rating_range_role(0, 15).is_none());
    }

    #[test]
    fn find_rating_range_returns_the_first_match() {
        let mut grm = GuildRoleManager::new();

        grm.add_guild(0);
        grm.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));
        grm.add_rating_range(0, RatingRange::new(345, Some(10), Some(30)));

        assert_eq!(grm.find_rating_range_role(0, 15), Some(123));
    }

    #[test]
    fn add_guild_resets_stored_roles() {
        let mut grm = GuildRoleManager::new();

        grm.add_guild(0);
        grm.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));

        assert_eq!(grm.find_rating_range_role(0, 15), Some(123));

        grm.add_guild(0);

        assert!(grm.find_rating_range_role(0, 15).is_none());
    }

    #[test]
    fn remove_role_can_be_called_on_an_empty_manager() {
        let mut grm = GuildRoleManager::new();

        grm.remove_role(0, 0);
    }

    #[test]
    fn remove_role_correctly_removes_roles() {
        let mut grm = GuildRoleManager::new();

        grm.add_guild(0);
        grm.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));

        assert_eq!(grm.find_rating_range_role(0, 15), Some(123));

        grm.remove_role(0, 123);

        assert!(grm.find_rating_range_role(0, 15).is_none());
    }

    #[test]
    fn remove_role_only_removes_the_first_role_with_a_specific_id() {
        let mut grm = GuildRoleManager::new();

        grm.add_guild(0);
        grm.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));
        grm.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));

        assert_eq!(grm.find_rating_range_role(0, 15), Some(123));

        grm.remove_role(0, 123);

        assert_eq!(grm.find_rating_range_role(0, 15), Some(123));
    }

    #[test]
    fn other_rating_range_roles_can_be_called_on_empty_manager() {
        let grm = GuildRoleManager::new();

        assert_eq!(grm.other_rating_range_roles(0, 0).len(), 0);
    }

    #[test]
    fn other_rating_range_roles_returns_other_roles() {
        let mut grm = GuildRoleManager::new();

        grm.add_guild(0);
        grm.add_rating_range(0, RatingRange::new(123, Some(10), Some(19)));
        grm.add_rating_range(0, RatingRange::new(345, Some(20), Some(30)));

        assert_eq!(grm.other_rating_range_roles(0, 123), vec![345]);
    }

    #[test]
    fn other_rating_range_filters_duplicates() {
        let mut grm = GuildRoleManager::new();

        grm.add_guild(0);
        grm.add_rating_range(0, RatingRange::new(123, Some(10), Some(19)));
        grm.add_rating_range(0, RatingRange::new(123, Some(50), Some(70)));
        grm.add_rating_range(0, RatingRange::new(345, Some(20), Some(30)));

        assert_eq!(grm.other_rating_range_roles(0, 123), vec![345]);
    }
}
