use std::{
    fmt::Display,
    str::FromStr,
};

use battler_wamp_values::{
    Value,
    WampDeserialize,
    WampDeserializeError,
    WampSerialize,
    WampSerializeError,
};
use serde_string_enum::{
    DeserializeStringEnum,
    SerializeStringEnum,
};

/// Authentication methods.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, SerializeStringEnum, DeserializeStringEnum,
)]
pub enum AuthMethod {
    /// WAMP Salted Challenge Response Authentication Mechanism.
    ///
    /// Password-based authentication method, where the shared secret is neither transmitted nor
    /// stored as cleartext.
    #[default]
    WampScram,
    /// Undisputed authentication.
    ///
    /// The client reports its authentication information and it is blindly accepted by the server.
    Undisputed,
}

impl TryFrom<&str> for AuthMethod {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "wamp-scram" => Ok(Self::WampScram),
            "wamp-battler-undisputed" => Ok(Self::Undisputed),
            _ => Err(Self::Error::msg(format!("invalid auth method: {value}"))),
        }
    }
}

impl FromStr for AuthMethod {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl Into<&'static str> for AuthMethod {
    fn into(self) -> &'static str {
        match self {
            Self::WampScram => "wamp-scram",
            Self::Undisputed => "wamp-battler-undisputed",
        }
    }
}

impl Into<String> for AuthMethod {
    fn into(self) -> String {
        Into::<&'static str>::into(self).to_owned()
    }
}

impl Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<&'static str>::into(*self))
    }
}

impl WampSerialize for AuthMethod {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        Ok(Value::String(self.into()))
    }
}

impl WampDeserialize for AuthMethod {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        value
            .string()
            .ok_or_else(|| WampDeserializeError::new("auth method must be a string"))?
            .try_into()
            .map_err(|err: anyhow::Error| WampDeserializeError::new(err.to_string()))
    }
}
