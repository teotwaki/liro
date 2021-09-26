use super::rating_range::RatingRange;
use crate::lichess::Format;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct RoleManager {
    guild_roles: Arc<Mutex<HashMap<u64, HashMap<u64, RatingRange>>>>,
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
    pub fn add_rating_range(&mut self, guild_id: u64, role_id: u64, rating: RatingRange) {
        trace!("RoleManager::add_rating_range() called");
        let mut lock = self.guild_roles.lock().unwrap();

        if let Some(gr) = lock.get_mut(&guild_id) {
            gr.insert(role_id, rating);
        } else {
            lock.insert(guild_id, [(role_id, rating)].iter().cloned().collect());
        }
    }

    pub fn remove_role(&mut self, guild_id: u64, role_id: u64) {
        trace!("RoleManager::remove_role() called");
        if let Some(gr) = self.guild_roles.lock().unwrap().get_mut(&guild_id) {
            gr.remove(&role_id);
        }
    }

    pub fn find_rating_range_roles(
        &self,
        guild_id: u64,
        ratings: &HashMap<Format, i16>,
    ) -> Vec<u64> {
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

    pub fn other_rating_range_roles<R>(&self, guild_id: u64, role_ids: R) -> Vec<u64>
    where
        R: AsRef<[u64]>,
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

    pub fn get_rating_role_names<R>(&self, guild_id: u64, role_ids: R) -> Vec<String>
    where
        R: AsRef<[u64]>,
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
            rm.find_rating_range_roles(0, &[(Format::Blitz, 15)].iter().cloned().collect())
                .len(),
            0
        );
    }

    #[test]
    fn find_rating_range_returns_all_ranges_that_match() {
        let mut rm = RoleManager::new();

        rm.add_rating_range(0, 123, RatingRange::new(Format::Blitz, Some(10), Some(20)));
        rm.add_rating_range(0, 345, RatingRange::new(Format::Blitz, Some(10), Some(30)));
        rm.add_rating_range(
            0,
            456,
            RatingRange::new(Format::Classical, Some(10), Some(30)),
        );

        let result =
            rm.find_rating_range_roles(0, &[(Format::Blitz, 15)].iter().cloned().collect());
        assert!(result.contains(&123));
        assert!(result.contains(&345));
        assert!(!result.contains(&456));
    }

    #[test]
    fn remove_role_can_be_called_on_an_empty_manager() {
        let mut rm = RoleManager::new();

        rm.remove_role(0, 0);
    }

    #[test]
    fn remove_role_correctly_removes_roles() {
        let mut rm = RoleManager::new();

        rm.add_rating_range(0, 123, RatingRange::new(Format::Blitz, Some(10), Some(20)));

        assert_eq!(
            rm.find_rating_range_roles(0, &[(Format::Blitz, 15)].iter().cloned().collect()),
            vec![123]
        );

        rm.remove_role(0, 123);

        assert_eq!(
            rm.find_rating_range_roles(0, &[(Format::Blitz, 15)].iter().cloned().collect())
                .len(),
            0
        );
    }

    #[test]
    fn other_rating_range_roles_can_be_called_on_empty_manager() {
        let rm = RoleManager::new();

        assert_eq!(rm.other_rating_range_roles(0, &[0]).len(), 0);
    }

    #[test]
    fn other_rating_range_roles_returns_other_roles() {
        let mut rm = RoleManager::new();

        rm.add_rating_range(0, 123, RatingRange::new(Format::Blitz, Some(10), Some(19)));
        rm.add_rating_range(0, 345, RatingRange::new(Format::Bullet, Some(20), Some(30)));

        assert_eq!(rm.other_rating_range_roles(0, &[123]), vec![345]);
    }
}
