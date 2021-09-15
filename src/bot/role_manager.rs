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
            under_re: Regex::new(r"^(U|u)(?P<max>\d{3,4})$").unwrap(),
            over_re: Regex::new(r"^(?P<min>\d{3,4})\+$").unwrap(),
            range_re: Regex::new(r"^(?P<min>\d{3,4})-(?P<max>\d{3,4})$").unwrap(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_rating_range_handles_exclusively_under() {
        let grr = GuildRoleManager::new();
        let m = grr.lock().await;
        let parsed = m.parse_rating_range(0, "U1000").unwrap();
        let rr = RatingRange::new(0, None, Some(999));

        assert_eq!(parsed, rr);
    }

    #[tokio::test]
    async fn parse_rating_range_handles_upper_and_lowercase() {
        let grr = GuildRoleManager::new();
        let m = grr.lock().await;
        let upper = m.parse_rating_range(0, "U1000");
        let lower = m.parse_rating_range(0, "u1000");

        assert_eq!(upper, lower);
    }

    #[tokio::test]
    async fn parse_rating_range_handles_exclusively_over() {
        let grr = GuildRoleManager::new();
        let m = grr.lock().await;
        let parsed = m.parse_rating_range(0, "2200+").unwrap();
        let rr = RatingRange::new(0, Some(2200), None);

        assert_eq!(parsed, rr);
    }

    #[tokio::test]
    async fn parse_rating_range_handles_in_between() {
        let grr = GuildRoleManager::new();
        let m = grr.lock().await;
        let parsed = m.parse_rating_range(0, "1000-1099").unwrap();
        let rr = RatingRange::new(0, Some(1000), Some(1099));

        assert_eq!(parsed, rr);
    }

    #[tokio::test]
    async fn parse_rating_range_ignores_random_strings() {
        let grr = GuildRoleManager::new();
        let m = grr.lock().await;

        assert!(m.parse_rating_range(0, "foo").is_none()); // random
        assert!(m.parse_rating_range(0, "2000++").is_none()); // invalid suffix
        assert!(m.parse_rating_range(0, "uu2000").is_none()); // invalid prefix
        assert!(m.parse_rating_range(0, "10-2000").is_none()); // value too small (10)
        assert!(m.parse_rating_range(0, "100-20000").is_none()); // value too big (20000)
    }

    #[tokio::test]
    async fn find_rating_range_role_can_be_called_on_an_empty_manager() {
        let grr = GuildRoleManager::new();
        let m = grr.lock().await;

        m.find_rating_range_role(0, 0);
    }

    #[tokio::test]
    async fn adding_roles_to_a_nonexistent_guild_does_nothing() {
        let grr = GuildRoleManager::new();
        let mut m = grr.lock().await;

        m.add_rating_range(0, RatingRange::new(0, Some(10), Some(20)));
        assert!(m.find_rating_range_role(0, 15).is_none());
    }

    #[tokio::test]
    async fn find_rating_range_returns_the_first_match() {
        let grr = GuildRoleManager::new();
        let mut m = grr.lock().await;

        m.add_guild(0);

        m.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));
        m.add_rating_range(0, RatingRange::new(345, Some(10), Some(30)));
        assert_eq!(m.find_rating_range_role(0, 15), Some(123));
    }

    #[tokio::test]
    async fn add_guild_resets_stored_roles() {
        let grr = GuildRoleManager::new();
        let mut m = grr.lock().await;

        m.add_guild(0);
        m.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));

        assert_eq!(m.find_rating_range_role(0, 15), Some(123));

        m.add_guild(0);

        assert!(m.find_rating_range_role(0, 15).is_none());
    }

    #[tokio::test]
    async fn remove_role_can_be_called_on_an_empty_manager() {
        let grr = GuildRoleManager::new();
        let mut m = grr.lock().await;

        m.remove_role(0, 0);
    }

    #[tokio::test]
    async fn remove_role_correctly_removes_roles() {
        let grr = GuildRoleManager::new();
        let mut m = grr.lock().await;

        m.add_guild(0);
        m.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));

        assert_eq!(m.find_rating_range_role(0, 15), Some(123));

        m.remove_role(0, 123);

        assert!(m.find_rating_range_role(0, 15).is_none());
    }

    #[tokio::test]
    async fn remove_role_only_removes_the_first_role_with_a_specific_id() {
        let grr = GuildRoleManager::new();
        let mut m = grr.lock().await;

        m.add_guild(0);
        m.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));
        m.add_rating_range(0, RatingRange::new(123, Some(10), Some(20)));

        assert_eq!(m.find_rating_range_role(0, 15), Some(123));

        m.remove_role(0, 123);

        assert_eq!(m.find_rating_range_role(0, 15), Some(123));
    }

    #[tokio::test]
    async fn other_rating_range_roles_can_be_called_on_empty_manager() {
        let grr = GuildRoleManager::new();
        let m = grr.lock().await;

        assert_eq!(m.other_rating_range_roles(0, 0).len(), 0);
    }

    #[tokio::test]
    async fn other_rating_range_roles_returns_other_roles() {
        let grr = GuildRoleManager::new();
        let mut m = grr.lock().await;

        m.add_guild(0);
        m.add_rating_range(0, RatingRange::new(123, Some(10), Some(19)));
        m.add_rating_range(0, RatingRange::new(345, Some(20), Some(30)));

        assert_eq!(m.other_rating_range_roles(0, 123), vec![345]);
    }

    #[tokio::test]
    async fn other_rating_range_filters_duplicates() {
        let grr = GuildRoleManager::new();
        let mut m = grr.lock().await;

        m.add_guild(0);
        m.add_rating_range(0, RatingRange::new(123, Some(10), Some(19)));
        m.add_rating_range(0, RatingRange::new(123, Some(50), Some(70)));
        m.add_rating_range(0, RatingRange::new(345, Some(20), Some(30)));

        assert_eq!(m.other_rating_range_roles(0, 123), vec![345]);
    }
}
