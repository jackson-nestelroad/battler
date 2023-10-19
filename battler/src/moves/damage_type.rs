use std::{
    fmt,
    fmt::Display,
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

/// The type of damage dealt by a move.
///
/// This type is only used for moves that deal static damage. If unset, moves deal damage based on a
/// series of calculations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageType {
    /// A set amount of damage.
    Set(u32),
    /// Damage equal to the target's level.
    Level,
}

impl Display for DamageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Set(value) => write!(f, "{value}"),
            Self::Level => write!(f, "level"),
        }
    }
}

impl From<u32> for DamageType {
    fn from(value: u32) -> Self {
        Self::Set(value)
    }
}

impl FromStr for DamageType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "level" => Ok(Self::Level),
            _ => Err(battler_error!("invalid damage type \"{s}\"")),
        }
    }
}

impl Serialize for DamageType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Set(n) => serializer.serialize_u32(*n),
            Self::Level => serializer.collect_str("level"),
        }
    }
}

struct DamageTypeVisitor;

impl<'de> Visitor<'de> for DamageTypeVisitor {
    type Value = DamageType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an integer or \"level\"")
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as u32))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_str(v)
            .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(&v), &self))
    }
}

impl<'de> Deserialize<'de> for DamageType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(DamageTypeVisitor)
    }
}

#[cfg(test)]
mod damage_type_tests {
    use crate::{
        common::test_serialization,
        moves::DamageType,
    };

    #[test]
    fn serializes_to_string() {
        test_serialization(DamageType::Level, "\"level\"");
        test_serialization(DamageType::Set(10), 10);
        test_serialization(DamageType::Set(44), 44);
    }
}
