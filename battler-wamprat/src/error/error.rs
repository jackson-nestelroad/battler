use battler_wamp::core::{
    error::WampError,
    uri::Uri,
};
use battler_wamp_values::{
    WampDeserializeError,
    WampSerializeError,
};
use thiserror::Error;

/// An error resulting from serializing a WAMP value to the expected format.
#[derive(Debug, Error)]
#[error("{msg}")]
pub struct WampratSerializeError {
    msg: String,
}

impl From<WampSerializeError> for WampratSerializeError {
    fn from(value: WampSerializeError) -> Self {
        Self {
            msg: value.to_string(),
        }
    }
}

impl Into<WampError> for WampratSerializeError {
    fn into(self) -> WampError {
        WampError::new(
            Uri::try_from("com.battler_wamprat.serialize_error").unwrap(),
            self.msg,
        )
    }
}

impl TryFrom<WampError> for WampratSerializeError {
    type Error = WampError;
    fn try_from(value: WampError) -> Result<Self, Self::Error> {
        if value.reason().as_ref() == "com.battler_wamprat.serialize_error" {
            Ok(Self {
                msg: value.message().to_owned(),
            })
        } else {
            Err(value)
        }
    }
}

/// An error resulting from deserializing a WAMP value to the expected format.
#[derive(Debug, Error)]
#[error("{msg}")]
pub struct WampratDeserializeError {
    msg: String,
}

impl From<WampDeserializeError> for WampratDeserializeError {
    fn from(value: WampDeserializeError) -> Self {
        Self {
            msg: value.to_string(),
        }
    }
}

impl Into<WampError> for WampratDeserializeError {
    fn into(self) -> WampError {
        WampError::new(
            Uri::try_from("com.battler_wamprat.deserialize_error").unwrap(),
            self.msg,
        )
    }
}

impl TryFrom<WampError> for WampratDeserializeError {
    type Error = WampError;
    fn try_from(value: WampError) -> Result<Self, Self::Error> {
        if value.reason().as_ref() == "com.battler_wamprat.deserialize_error" {
            Ok(Self {
                msg: value.message().to_owned(),
            })
        } else {
            Err(value)
        }
    }
}
