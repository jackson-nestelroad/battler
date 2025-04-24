use std::{
    fmt,
    fmt::Display,
    str::FromStr,
};

use anyhow::Error;
use serde::{
    de::{
        Unexpected,
        Visitor,
    },
    Deserialize,
    Serialize,
    Serializer,
};

use crate::{
    error::general_error,
    mons::Type,
};

/// The type of one-hit KO dealt by the move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OhkoType {
    /// OHKOs any target.
    Always,
    /// OHKOs targets of the given type.
    Type(Type),
}

impl Display for OhkoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Always => write!(f, "{}", true),
            Self::Type(typ) => write!(f, "{typ}"),
        }
    }
}

impl FromStr for OhkoType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(OhkoType::Type(Type::from_str(s).map_err(general_error)?))
    }
}

impl Serialize for OhkoType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Always => serializer.serialize_bool(true),
            Self::Type(typ) => typ.serialize(serializer),
        }
    }
}

struct OhkoTypeVisitor;

impl<'de> Visitor<'de> for OhkoTypeVisitor {
    type Value = OhkoType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "true or \"level\"")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if !v {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        } else {
            Ok(Self::Value::Always)
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_str(v).map_err(|_| E::invalid_value(Unexpected::Str(&v), &self))
    }
}

impl<'de> Deserialize<'de> for OhkoType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(OhkoTypeVisitor)
    }
}

#[cfg(test)]
mod ohko_type_tests {
    use crate::{
        common::test_serialization,
        mons::Type,
        moves::OhkoType,
    };

    #[test]
    fn serializes_to_string() {
        test_serialization(OhkoType::Always, true);
        test_serialization(OhkoType::Type(Type::Ice), "\"Ice\"");
    }
}
