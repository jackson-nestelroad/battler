use std::{
    fmt,
    str::FromStr,
};

use serde::{
    de::Visitor,
    Deserialize,
    Serialize,
};

use crate::{
    battler_error,
    common::Error,
};

/// The chance that a Mon is shiny.
#[derive(Debug, Default, Clone, PartialEq)]
pub enum ShinyChance {
    /// The Mon cannot be shiny.
    Never,
    /// The Mon may be shiny.
    #[default]
    Chance,
    /// The Mon must be shiny.
    Always,
}

impl From<bool> for ShinyChance {
    fn from(value: bool) -> Self {
        if value {
            Self::Always
        } else {
            Self::Never
        }
    }
}

impl FromStr for ShinyChance {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "maybe" => Ok(Self::Chance),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err(battler_error!("invalid shiny chance \"{s}\"")),
        }
    }
}

impl Serialize for ShinyChance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Never => serializer.serialize_bool(false),
            Self::Chance => serializer.serialize_str("maybe"),
            Self::Always => serializer.serialize_bool(true),
        }
    }
}

struct ShinyChanceVisitor;

impl<'de> Visitor<'de> for ShinyChanceVisitor {
    type Value = ShinyChance;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a boolean or \"maybe\"")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_str(v)
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(&v), &self))
    }
}

impl<'de> Deserialize<'de> for ShinyChance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ShinyChanceVisitor)
    }
}

#[cfg(test)]
mod shiny_chance_tests {
    use crate::{
        common::{
            test_serialization,
            test_string_deserialization,
        },
        mons::ShinyChance,
    };

    #[test]
    fn serializes_numbers_and_strings() {
        test_serialization(ShinyChance::Never, false);
        test_serialization(ShinyChance::Chance, "\"maybe\"");
        test_serialization(ShinyChance::Always, true);
    }

    #[test]
    fn deserializes_alias_strings() {
        test_string_deserialization("always", ShinyChance::Always);
        test_string_deserialization("never", ShinyChance::Never);
    }
}
