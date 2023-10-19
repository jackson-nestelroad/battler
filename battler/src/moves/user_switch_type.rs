use std::{
    fmt,
    fmt::Display,
    str::FromStr,
};

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
    battler_error,
    common::Error,
};

/// The type of user switch performed when using a move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserSwitchType {
    /// Normal switch out.
    Normal,
    /// Switch out that copies all volatile effects to the replacement Mon.
    CopyVolatile,
}

impl Display for UserSwitchType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "{}", true),
            Self::CopyVolatile => write!(f, "copyvolatile"),
        }
    }
}

impl FromStr for UserSwitchType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "copyvolatile" => Ok(Self::CopyVolatile),
            _ => Err(battler_error!("invalid user switch type: \"{s}\"")),
        }
    }
}

impl Serialize for UserSwitchType {
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
    type Value = UserSwitchType;

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

impl<'de> Deserialize<'de> for UserSwitchType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(UserSwitchTypeVisitor)
    }
}

#[cfg(test)]
mod user_switch_type_tests {
    use crate::{
        common::test_serialization,
        moves::UserSwitchType,
    };

    #[test]
    fn serializes_to_string() {
        test_serialization(UserSwitchType::Normal, true);
        test_serialization(UserSwitchType::CopyVolatile, "\"copyvolatile\"");
    }
}
