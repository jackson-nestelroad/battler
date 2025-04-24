use anyhow::Error;
use battler_wamp_values::{
    Value,
    WampDeserialize,
    WampDictionary,
};

use crate::{
    core::{
        hash::HashSet,
        id::Id,
    },
    message::message::PublishMessage,
};

/// Options for publishing an event.
#[derive(Debug, Default, Clone, PartialEq, Eq, WampDictionary)]
pub struct PublishOptions {
    /// Should the publisher be excluded from receiving the event?
    #[battler_wamp_values(default)]
    pub exclude_me: bool,
    /// Blocked session IDs.
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub exclude: Option<HashSet<Id>>,
    /// Blocked authenticated IDs.
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub exclude_authid: Option<HashSet<String>>,
    /// Blocked authenticated roles.
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub exclude_authrole: Option<HashSet<String>>,
    /// Allowed session IDs.
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub eligible: Option<HashSet<Id>>,
    /// Allowed authenticated IDs.
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub eligible_authid: Option<HashSet<String>>,
    /// Allowed authenticated roles.
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub eligible_authrole: Option<HashSet<String>>,
}

impl TryFrom<&PublishMessage> for PublishOptions {
    type Error = Error;
    fn try_from(value: &PublishMessage) -> Result<Self, Self::Error> {
        Self::wamp_deserialize(Value::Dictionary(value.options.clone())).map_err(|err| err.into())
    }
}
