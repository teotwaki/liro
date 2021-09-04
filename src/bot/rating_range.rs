use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct RatingRange {
    role_id: u64,
    min: Option<i16>,
    max: Option<i16>,
}

impl RatingRange {
    pub fn new(role_id: u64, min: Option<i16>, max: Option<i16>) -> RatingRange {
        let rr = RatingRange { role_id, min, max };
        debug!("Creating new {}", rr);
        rr
    }

    pub fn is_match(&self, rating: i16) -> bool {
        match (self.min, self.max) {
            (Some(min), Some(max)) => rating >= min && rating <= max,
            (Some(min), None) => rating >= min,
            (None, Some(max)) => rating < max,
            _ => false,
        }
    }

    pub fn role_id(&self) -> u64 {
        self.role_id
    }
}

impl fmt::Display for RatingRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.min, self.max) {
            (Some(min), Some(max)) => write!(
                f,
                "RatingRange<role_id={} min={} max={}",
                self.role_id, min, max
            ),
            (Some(min), None) => write!(
                f,
                "RatingRange<role_id={} min={} max=None",
                self.role_id, min
            ),
            (None, Some(max)) => write!(
                f,
                "RatingRange<role_id={} min=None max={}",
                self.role_id, max
            ),
            (None, None) => write!(f, "RatingRange<role_id={} min=None max=None", self.role_id),
        }
    }
}
