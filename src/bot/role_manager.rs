use serenity::all::{GuildId, RoleId};

use super::rating_range::RatingRange;
use crate::lichess::Format;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct RoleManager {
    guild_roles: Arc<Mutex<HashMap<GuildId, HashMap<RoleId, RatingRange>>>>,
}

impl RoleManager {
    pub fn new() -> Self {
        trace!("RoleManager::new() called");
        RoleManager {
            guild_roles: Default::default(),
        }
    }

    /// Adds a new rating range role for the specific `guild_id`
    ///
    /// If the `guild_id` does not exist in the role manager, it is automatically created.
    pub fn add_rating_range<R>(&mut self, guild_id: GuildId, role_id: RoleId, rating: R)
    where
        R: Into<RatingRange>,
    {
        trace!("RoleManager::add_rating_range() called");
        let mut lock = self.guild_roles.lock().unwrap();

        if let Some(gr) = lock.get_mut(&guild_id) {
            gr.insert(role_id, rating.into());
        } else {
            lock.insert(
                guild_id,
                [(role_id, rating.into())].iter().cloned().collect(),
            );
        }
    }

    pub fn remove_role(&mut self, guild_id: GuildId, role_id: RoleId) {
        trace!("RoleManager::remove_role() called");
        if let Some(gr) = self.guild_roles.lock().unwrap().get_mut(&guild_id) {
            gr.remove(&role_id);
        }
    }

    pub fn delete_guild(&mut self, guild_id: GuildId) {
        trace!("RoleManager::delete_guild() called");
        self.guild_roles.lock().unwrap().remove(&guild_id);
    }

    pub fn find_rating_range_roles(
        &self,
        guild_id: GuildId,
        ratings: &HashMap<Format, i16>,
    ) -> Vec<RoleId> {
        trace!("RoleManager::find_rating_range_role() called");
        self.guild_roles
            .lock()
            .unwrap()
            .get(&guild_id)
            .map(|gr| {
                gr.iter()
                    .filter_map(|(&k, v)| {
                        for (format, rating) in ratings {
                            if v.is_match(*format, *rating) {
                                return Some(k);
                            }
                        }
                        None
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn other_rating_range_roles<R>(&self, guild_id: GuildId, role_ids: R) -> Vec<RoleId>
    where
        R: AsRef<[RoleId]>,
    {
        trace!("RoleManager::other_rating_range_roles() called");
        self.guild_roles
            .lock()
            .unwrap()
            .get(&guild_id)
            .map(|gr| {
                gr.keys()
                    .filter_map(|k| {
                        if role_ids.as_ref().contains(k) {
                            None
                        } else {
                            Some(*k)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_rating_role_names<R>(&self, guild_id: GuildId, role_ids: R) -> Vec<String>
    where
        R: AsRef<[RoleId]>,
    {
        trace!("RoleManager::get_rating_role_names() called");
        self.guild_roles
            .lock()
            .unwrap()
            .get(&guild_id)
            .map(|gr| {
                gr.iter()
                    .filter_map(|(k, v)| {
                        if role_ids.as_ref().contains(k) {
                            v.get_name()
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_rating_range_role_can_be_called_on_an_empty_manager() {
        let rm = RoleManager::new();

        assert_eq!(
            rm.find_rating_range_roles(
                GuildId::new(1),
                &[(Format::Blitz, 15)].iter().cloned().collect()
            )
            .len(),
            0
        );
    }

    #[test]
    fn find_rating_range_returns_all_ranges_that_match() {
        let mut rm = RoleManager::new();
        let gid = GuildId::new(1);
        let roles = [RoleId::new(123), RoleId::new(345), RoleId::new(456)];

        rm.add_rating_range(
            gid,
            RoleId::new(123),
            RatingRange::new(Format::Blitz, Some(10), Some(20)),
        );
        rm.add_rating_range(
            gid,
            RoleId::new(345),
            RatingRange::new(Format::Blitz, Some(10), Some(30)),
        );
        rm.add_rating_range(
            gid,
            RoleId::new(456),
            RatingRange::new(Format::Classical, Some(10), Some(30)),
        );

        let result =
            rm.find_rating_range_roles(gid, &[(Format::Blitz, 15)].iter().cloned().collect());
        assert!(result.contains(&roles[0]));
        assert!(result.contains(&roles[1]));
        assert!(!result.contains(&roles[2]));
    }

    #[test]
    fn remove_role_can_be_called_on_an_empty_manager() {
        let mut rm = RoleManager::new();

        rm.remove_role(GuildId::new(1), RoleId::new(1));
    }

    #[test]
    fn remove_role_correctly_removes_roles() {
        let mut rm = RoleManager::new();
        let gid = GuildId::new(1);
        let rid = RoleId::new(123);

        rm.add_rating_range(
            gid,
            rid,
            RatingRange::new(Format::Blitz, Some(10), Some(20)),
        );

        assert_eq!(
            rm.find_rating_range_roles(gid, &[(Format::Blitz, 15)].iter().cloned().collect()),
            vec![rid]
        );

        rm.remove_role(gid, rid);

        assert_eq!(
            rm.find_rating_range_roles(gid, &[(Format::Blitz, 15)].iter().cloned().collect())
                .len(),
            0
        );
    }

    #[test]
    fn other_rating_range_roles_can_be_called_on_empty_manager() {
        let rm = RoleManager::new();

        assert_eq!(
            rm.other_rating_range_roles(GuildId::new(1), [RoleId::new(1)])
                .len(),
            0
        );
    }

    #[test]
    fn other_rating_range_roles_returns_other_roles() {
        let mut rm = RoleManager::new();
        let gid = GuildId::new(1);
        let roles = [RoleId::new(123), RoleId::new(345), RoleId::new(456)];

        rm.add_rating_range(
            gid,
            roles[0],
            RatingRange::new(Format::Blitz, Some(10), Some(19)),
        );
        rm.add_rating_range(
            gid,
            roles[1],
            RatingRange::new(Format::Bullet, Some(20), Some(30)),
        );

        assert_eq!(rm.other_rating_range_roles(gid, [roles[0]]), vec![roles[1]]);
    }
}
