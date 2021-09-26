use crate::lichess::Format;
use lazy_static::lazy_static;
use regex::Regex;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatingRange {
    format: Format,
    min: Option<i16>,
    max: Option<i16>,
}

impl RatingRange {
    pub fn new<F, O>(format: F, min: O, max: O) -> RatingRange
    where
        F: Into<Format>,
        O: Into<Option<i16>>,
    {
        trace!("RatingRange::new() called");
        let rr = RatingRange {
            format: format.into(),
            min: min.into(),
            max: max.into(),
        };
        debug!("Creating new {}", rr);
        rr
    }

    pub fn is_match<F>(&self, format: F, rating: i16) -> bool
    where
        F: Into<Format>,
    {
        trace!("RatingRange::is_match() called");
        let format = format.into();

        if self.format != format {
            return false;
        }

        match (self.min, self.max) {
            (Some(min), Some(max)) => rating >= min && rating <= max,
            (Some(min), None) => rating >= min,
            (None, Some(max)) => rating < max,
            _ => false,
        }
    }

    pub fn get_name(&self) -> Option<String> {
        match (self.min, self.max) {
            (Some(min), Some(max)) => Some(format!(
                "{}-{} {}",
                min,
                max,
                self.format.to_string().to_lowercase()
            )),
            (Some(min), None) => Some(format!(
                "{}+ {}",
                min,
                self.format.to_string().to_lowercase()
            )),
            (None, Some(max)) => Some(format!(
                "U{} {}",
                max,
                self.format.to_string().to_lowercase()
            )),
            _ => None,
        }
    }
}

impl fmt::Display for RatingRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        trace!("RatingRange::fmt() called");
        write!(f, "RatingRange<min={:?} max={:?}>", self.min, self.max)
    }
}

impl FromStr for RatingRange {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(((U|u)(?P<under>\d{3,4}))|((?P<min>\d{3,4})(\+|-(?P<max>\d{3,4})))) (?P<format>\w+)$").unwrap();
        }

        let captures = RE.captures(s).ok_or(())?;
        let format = captures
            .name("format")
            .map(|m| m.as_str().parse::<Format>())
            .ok_or(())??;

        let (min, max) = match (
            captures.name("min"),
            captures.name("max"),
            captures.name("under"),
        ) {
            (Some(min), None, _) => (min.as_str().parse().ok(), None),
            (Some(min), Some(max), _) => (min.as_str().parse().ok(), max.as_str().parse().ok()),
            (_, _, Some(max)) => (None, max.as_str().parse().ok()),
            _ => unreachable!(),
        };

        Ok(RatingRange::new(format, min, max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // This test is for ranges like U1000
    fn is_match_recognises_exclusively_under() {
        let f = Format::Blitz;
        let rr = RatingRange::new(f, None, Some(10));

        assert!(rr.is_match(f, 9));
        assert!(!rr.is_match(f, 10));
        assert!(!rr.is_match(f, 11));
    }

    #[test]
    // This test is for ranges like 2200+
    fn is_match_recognises_exclusively_over() {
        let f = Format::Blitz;
        let rr = RatingRange::new(f, Some(10), None);

        assert!(!rr.is_match(f, 9));
        assert!(rr.is_match(f, 10));
        assert!(rr.is_match(f, 11));
    }

    #[test]
    // This test is for ranges like 1000-1099 or 1400-1699
    fn is_match_recognises_in_between() {
        let f = Format::Blitz;
        let rr = RatingRange::new(f, Some(10), Some(19));

        assert!(!rr.is_match(f, 9));
        assert!(rr.is_match(f, 10));
        assert!(rr.is_match(f, 19));
        assert!(!rr.is_match(f, 20));
    }

    #[test]
    fn is_match_differentiates_on_format() {
        let rr = RatingRange::new(Format::Blitz, Some(10), Some(19));

        assert!(!rr.is_match(Format::Bullet, 15));
    }

    #[test]
    fn parse_correctly_detects_format() {
        assert_eq!(
            "2200+ classical".parse::<RatingRange>().unwrap().format,
            Format::Classical
        );
        assert_eq!(
            "2200+ blitz".parse::<RatingRange>().unwrap().format,
            Format::Blitz
        );
        assert_eq!(
            "2200+ bullet".parse::<RatingRange>().unwrap().format,
            Format::Bullet
        );
        assert_eq!(
            "2200+ rapid".parse::<RatingRange>().unwrap().format,
            Format::Rapid
        );
    }

    #[test]
    fn parse_correctly_handles_under() {
        let rr = "U3000 blitz".parse::<RatingRange>().unwrap();

        assert_eq!(rr.min, None);
        assert_eq!(rr.max, Some(3000));
        assert!(rr.is_match(Format::Blitz, 2999));
        assert!(!rr.is_match(Format::Blitz, 3000));
    }

    #[test]
    fn parse_correctly_handles_over() {
        let rr = "2200+ blitz".parse::<RatingRange>().unwrap();

        assert_eq!(rr.min, Some(2200));
        assert_eq!(rr.max, None);
        assert!(rr.is_match(Format::Blitz, 2300));
        assert!(rr.is_match(Format::Blitz, 2200));
        assert!(!rr.is_match(Format::Blitz, 2199));
    }

    #[test]
    fn parse_correctly_handles_between() {
        let rr = "1400-1599 bullet".parse::<RatingRange>().unwrap();

        assert_eq!(rr.min, Some(1400));
        assert_eq!(rr.max, Some(1599));

        assert!(!rr.is_match(Format::Bullet, 1399));
        assert!(rr.is_match(Format::Bullet, 1400));
        assert!(rr.is_match(Format::Bullet, 1599));
        assert!(!rr.is_match(Format::Bullet, 1600));
    }
}
