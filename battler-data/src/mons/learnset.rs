use std::{
    fmt,
    fmt::Display,
    str::FromStr,
};

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::Error;
use serde_string_enum::{
    DeserializeStringEnum,
    SerializeStringEnum,
};

/// The source of a move, which details how a species can learn a move in their learnset.
///
/// This enum is encoded as a single letter followed by optional details:
/// - `Machine`: `M`
/// - `Tutor`: `T`
/// - `Level`: `L#`, where `#` is the level number.
/// - `Egg`: `E`
/// - `Restricted`: `R`
#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeStringEnum, DeserializeStringEnum)]
pub enum MoveSource {
    /// Taught manually by a Technical or Hidden Machine.
    Machine,
    /// Taught manually by a Move Tutor.
    Tutor,
    /// Learned on level up at the specified level.
    Level(u8),
    /// Learned only through breeding.
    Egg,
    /// Restricted to some forme.
    Restricted,
}

impl FromStr for MoveSource {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &s[0..1] {
            "M" => Ok(Self::Machine),
            "T" => Ok(Self::Tutor),
            "L" => {
                let level_str = &s[1..];
                let level = level_str.parse::<u8>().map_err(|err| {
                    Into::<Error>::into(err).context(format!("invalid level: {level_str}"))
                })?;
                Ok(Self::Level(level))
            }
            "E" => Ok(Self::Egg),
            "R" => Ok(Self::Restricted),
            _ => Err(Error::msg(format!("invalid move source: {s}"))),
        }
    }
}

impl Display for MoveSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Machine => write!(f, "M"),
            Self::Tutor => write!(f, "T"),
            Self::Level(level) => write!(f, "L{level}"),
            Self::Egg => write!(f, "E"),
            Self::Restricted => write!(f, "R"),
        }
    }
}

/// A species learnset, which maps move names to how they are learned.
pub type LearnSet = HashMap<String, HashSet<MoveSource>>;

#[cfg(test)]
mod move_source_test {
    use crate::{
        MoveSource,
        test_util::test_string_serialization,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(MoveSource::Machine, "M");
        test_string_serialization(MoveSource::Tutor, "T");
        test_string_serialization(MoveSource::Level(10), "L10");
        test_string_serialization(MoveSource::Level(25), "L25");
        test_string_serialization(MoveSource::Egg, "E");
    }
}
