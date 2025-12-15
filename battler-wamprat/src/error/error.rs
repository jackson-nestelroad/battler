use battler_wamp::core::error::WampError;
use battler_wamp_uri::Uri;
use battler_wamp_values::{
    WampDeserializeError,
    WampSerializeError,
};
use thiserror::Error;

/// An error resulting from serializing a value to the expected format.
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
            self.to_string(),
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

/// An error resulting from a procedure invocation missing the original procedure called by the
/// caller.
#[derive(Debug, Error)]
#[error("invocation is missing called procedure")]
pub struct WampratInvocationMissingProcedure;

impl From<WampDeserializeError> for WampratInvocationMissingProcedure {
    fn from(_: WampDeserializeError) -> Self {
        Self
    }
}

impl Into<WampError> for WampratInvocationMissingProcedure {
    fn into(self) -> WampError {
        WampError::new(
            Uri::try_from("com.battler_wamprat.invocation_missing_procedure").unwrap(),
            self.to_string(),
        )
    }
}

impl TryFrom<WampError> for WampratInvocationMissingProcedure {
    type Error = WampError;
    fn try_from(value: WampError) -> Result<Self, Self::Error> {
        if value.reason().as_ref() == "com.battler_wamprat.invocation_missing_procedure" {
            Ok(Self)
        } else {
            Err(value)
        }
    }
}

/// An error resulting from a topic event missing the topic published by the publisher.
#[derive(Debug, Error)]
#[error("event is missing published topic")]
pub struct WampratEventMissingTopic;

impl From<WampDeserializeError> for WampratEventMissingTopic {
    fn from(_: WampDeserializeError) -> Self {
        Self
    }
}

impl Into<WampError> for WampratEventMissingTopic {
    fn into(self) -> WampError {
        WampError::new(
            Uri::try_from("com.battler_wamprat.event_missing_topic").unwrap(),
            self.to_string(),
        )
    }
}

impl TryFrom<WampError> for WampratEventMissingTopic {
    type Error = WampError;
    fn try_from(value: WampError) -> Result<Self, Self::Error> {
        if value.reason().as_ref() == "com.battler_wamprat.event_missing_topic" {
            Ok(Self)
        } else {
            Err(value)
        }
    }
}
