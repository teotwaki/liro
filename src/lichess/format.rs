use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Blitz,
    Bullet,
    Classical,
    Rapid,
}

impl FromStr for Format {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blitz" => Ok(Format::Blitz),
            "bullet" => Ok(Format::Bullet),
            "classical" => Ok(Format::Classical),
            "rapid" => Ok(Format::Rapid),
            _ => Err(()),
        }
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Format::Blitz => "Blitz",
            Format::Bullet => "Bullet",
            Format::Classical => "Classical",
            Format::Rapid => "Rapid",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_can_be_compared() {
        assert_ne!(Format::Blitz, Format::Bullet);
        assert_ne!(Format::Bullet, Format::Rapid);
        assert_ne!(Format::Bullet, Format::Rapid);
    }

    #[test]
    fn format_can_be_parsed_from_string() {
        assert_eq!("classical".parse::<Format>().unwrap(), Format::Classical);
        assert_eq!("blitz".parse::<Format>().unwrap(), Format::Blitz);
        assert_eq!("bullet".parse::<Format>().unwrap(), Format::Bullet);
        assert_eq!("rapid".parse::<Format>().unwrap(), Format::Rapid);
    }

    #[test]
    fn format_parser_reports_error() {
        assert!("foo".parse::<Format>().is_err());
    }
}
