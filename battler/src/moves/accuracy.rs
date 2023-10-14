use std::{
    fmt,
    str::FromStr,
};

use serde::{
    de::Visitor,
    Deserialize,
    Serialize,
    Serializer,
};

use crate::{
    battler_error,
    common::Error,
};

/// The base accuracy of a move.
#[derive(Debug, Clone, PartialEq)]
pub enum Accuracy {
    /// The base chance for the move to hit.
    Chance(u8),
    /// The move will always hit.
    Always,
    /// The move is exempt from accuracy checks.
    Exempt,
}

impl From<u8> for Accuracy {
    fn from(value: u8) -> Self {
        Self::Chance(value)
    }
}

impl FromStr for Accuracy {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "always" => Ok(Self::Always),
            "exempt" => Ok(Self::Exempt),
            _ => Err(battler_error!("invalid accuracy \"{s}\"")),
        }
    }
}

impl Serialize for Accuracy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Chance(n) => serializer.serialize_u8(*n),
            Self::Always => serializer.collect_str("always"),
            Self::Exempt => serializer.collect_str("exempt"),
        }
    }
}

struct AccuracyVisitor;

impl<'de> Visitor<'de> for AccuracyVisitor {
    type Value = Accuracy;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an integer, \"always\", or \"exempt\"")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as u8))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_str(v)
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(&v), &self))
    }
}

impl<'de> Deserialize<'de> for Accuracy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(AccuracyVisitor)
    }
}

#[cfg(test)]
mod accuracy_tests {
    use crate::{
        common::test_serialization,
        moves::Accuracy,
    };

    #[test]
    fn serializes_numbers_and_strings() {
        test_serialization(Accuracy::Chance(100), 100);
        test_serialization(Accuracy::Chance(50), 50);
        test_serialization(Accuracy::Always, "\"always\"");
        test_serialization(Accuracy::Exempt, "\"exempt\"");
    }
}
