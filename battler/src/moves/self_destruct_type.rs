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

use crate::error::general_error;

/// How the user self destructs after a move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfDestructType {
    /// The user always self destructs.
    Always,
    /// The user only self destructs if the move hit.
    IfHit,
}

impl Display for SelfDestructType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Always => write!(f, "{}", true),
            Self::IfHit => write!(f, "ifhit"),
        }
    }
}

impl FromStr for SelfDestructType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ifhit" => Ok(Self::IfHit),
            _ => Err(general_error(format!(
                "invalid self destruct type: \"{s}\""
            ))),
        }
    }
}

impl Serialize for SelfDestructType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Always => serializer.serialize_bool(true),
            Self::IfHit => serializer.serialize_str("ifhit"),
        }
    }
}

struct SelfDestructTypeVisitor;

impl<'de> Visitor<'de> for SelfDestructTypeVisitor {
    type Value = SelfDestructType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "true or \"ifhit\"")
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

impl<'de> Deserialize<'de> for SelfDestructType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(SelfDestructTypeVisitor)
    }
}

#[cfg(test)]
mod self_switch_type_tests {
    use crate::{
        common::test_serialization,
        moves::SelfDestructType,
    };

    #[test]
    fn serializes_to_string() {
        test_serialization(SelfDestructType::Always, true);
        test_serialization(SelfDestructType::IfHit, "\"ifhit\"");
    }
}
