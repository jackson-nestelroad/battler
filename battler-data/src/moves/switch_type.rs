use std::{
    fmt::{
        self,
        Display,
    },
    str::FromStr,
};

use anyhow::Error;
use serde::{
    Deserialize,
    Serialize,
    Serializer,
    de::{
        Unexpected,
        Visitor,
    },
};

/// The type of user switch performed when using a move.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SwitchType {
    /// Normal switch out.
    #[default]
    Normal,
    /// Switch out that copies all volatile effects to the replacement Mon.
    CopyVolatile,
}

impl Display for SwitchType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "{}", true),
            Self::CopyVolatile => write!(f, "copyvolatile"),
        }
    }
}

impl FromStr for SwitchType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "copyvolatile" => Ok(Self::CopyVolatile),
            _ => Err(Error::msg(format!("invalid user switch type: \"{s}\""))),
        }
    }
}

impl Serialize for SwitchType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Normal => serializer.serialize_bool(true),
            Self::CopyVolatile => serializer.serialize_str("copyvolatile"),
        }
    }
}

struct UserSwitchTypeVisitor;

impl<'de> Visitor<'de> for UserSwitchTypeVisitor {
    type Value = SwitchType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "true or \"copyvolatile\"")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if !v {
            Err(E::invalid_value(Unexpected::Bool(v), &self))
        } else {
            Ok(Self::Value::Normal)
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_str(v).map_err(|_| E::invalid_value(Unexpected::Str(&v), &self))
    }
}

impl<'de> Deserialize<'de> for SwitchType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(UserSwitchTypeVisitor)
    }
}

#[cfg(test)]
mod user_switch_type_test {
    use crate::{
        moves::SwitchType,
        test_util::test_serialization,
    };

    #[test]
    fn serializes_to_string() {
        test_serialization(SwitchType::Normal, true);
        test_serialization(SwitchType::CopyVolatile, "\"copyvolatile\"");
    }
}
