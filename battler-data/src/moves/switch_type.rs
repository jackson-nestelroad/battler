use alloc::format;
use core::{
    fmt,
    fmt::Display,
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

/// The type of volatile copy to perform.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CopyVolatileType {
    #[default]
    AllCopyable,
    SubstituteOnly,
}

/// The type of user switch performed when using a move.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SwitchType {
    /// Normal switch out.
    #[default]
    Normal,
    /// Switch out that copies all volatile effects to the replacement Mon.
    CopyVolatile(CopyVolatileType),
    /// Normal switch out if the move hit.
    IfHit,
}

impl SwitchType {
    /// The corresponding [`CopyVolatileType`], if any.
    pub fn copy_volatile(&self) -> Option<CopyVolatileType> {
        match self {
            Self::CopyVolatile(copy_volatile_type) => Some(*copy_volatile_type),
            _ => None,
        }
    }

    /// Does the switch depend on hitting the target?
    pub fn if_hit(&self) -> bool {
        match self {
            Self::IfHit => true,
            _ => false,
        }
    }
}

impl Display for SwitchType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "{}", true),
            Self::CopyVolatile(CopyVolatileType::AllCopyable) => write!(f, "copyvolatile"),
            Self::CopyVolatile(CopyVolatileType::SubstituteOnly) => write!(f, "copysubstitute"),
            Self::IfHit => write!(f, "ifhit"),
        }
    }
}

impl FromStr for SwitchType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "copyvolatile" => Ok(Self::CopyVolatile(CopyVolatileType::AllCopyable)),
            "copysubstitute" => Ok(Self::CopyVolatile(CopyVolatileType::SubstituteOnly)),
            "ifhit" => Ok(Self::IfHit),
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
            Self::CopyVolatile(CopyVolatileType::AllCopyable) => {
                serializer.serialize_str("copyvolatile")
            }
            Self::CopyVolatile(CopyVolatileType::SubstituteOnly) => {
                serializer.serialize_str("copysubstitute")
            }
            Self::IfHit => serializer.serialize_str("ifhit"),
        }
    }
}

struct UserSwitchTypeVisitor;

impl<'de> Visitor<'de> for UserSwitchTypeVisitor {
    type Value = SwitchType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "true, \"copyvolatile\", \"copysubstitute\", or \"ifhit\""
        )
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
        moves::{
            CopyVolatileType,
            SwitchType,
        },
        test_util::test_serialization,
    };

    #[test]
    fn serializes_to_string() {
        test_serialization(SwitchType::Normal, true);
        test_serialization(
            SwitchType::CopyVolatile(CopyVolatileType::AllCopyable),
            "\"copyvolatile\"",
        );
        test_serialization(
            SwitchType::CopyVolatile(CopyVolatileType::SubstituteOnly),
            "\"copysubstitute\"",
        );
        test_serialization(SwitchType::IfHit, "\"ifhit\"");
    }
}
