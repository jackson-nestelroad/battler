use std::fmt::Display;

use battler_wamp_values::{
    Value,
    WampDeserialize,
    WampDeserializeError,
    WampSerialize,
    WampSerializeError,
};

/// TLS channel binding. Binds authentication at application layers to secure sessions at lower
/// layers in the network stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelBinding {
    /// RFC5929.
    TlsUnique,
    /// RFC9266.
    TlsServerEndPoint,
}

impl TryFrom<&str> for ChannelBinding {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "tls-unique" => Ok(Self::TlsUnique),
            "tls-server-end-point" => Ok(Self::TlsServerEndPoint),
            _ => Err(Self::Error::msg(format!(
                "invalid channel binding: {value}"
            ))),
        }
    }
}

impl Into<&'static str> for ChannelBinding {
    fn into(self) -> &'static str {
        match self {
            Self::TlsUnique => "tls-unique",
            Self::TlsServerEndPoint => "tls-server-end-point",
        }
    }
}

impl Into<String> for ChannelBinding {
    fn into(self) -> String {
        Into::<&'static str>::into(self).to_owned()
    }
}

impl Display for ChannelBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<&'static str>::into(*self))
    }
}

impl WampSerialize for ChannelBinding {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        Ok(Value::String(self.into()))
    }
}

impl WampDeserialize for ChannelBinding {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        value
            .string()
            .ok_or_else(|| WampDeserializeError::new("channel binding must be a string"))?
            .try_into()
            .map_err(|err: anyhow::Error| WampDeserializeError::new(err.to_string()))
    }
}
