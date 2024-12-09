use std::fmt::Debug;

use anyhow::Result;

use crate::{
    core::uri::Uri,
    message::message::Message,
    serializer::{
        json::JsonSerializer,
        message_pack::MessagePackSerializer,
    },
};

/// The type of serializer to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SerializerType {
    /// Serializes messages to and from JavaScript Object Notation.
    Json,
    /// Serializes messages to and from the MessagePack format.
    MessagePack,
}

impl SerializerType {
    /// The protocol URI used during protocol negotiation.
    pub fn uri(&self) -> Uri {
        match self {
            Self::Json => Uri::from_known("wamp.2.json"),
            Self::MessagePack => Uri::from_known("wamp.2.msgpack"),
        }
    }
}

impl TryFrom<&str> for SerializerType {
    type Error = &'static str;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "wamp.2.json" => Ok(Self::Json),
            "wamp.2.msgpack" => Ok(Self::MessagePack),
            _ => Err("unsupported serializer"),
        }
    }
}

/// A serializer, which serializes and deserializes WAMP messages to a well-known format that can be
/// passed over wire.
///
/// Does not implement message batching.
pub trait Serializer: Send + Debug {
    /// Serializes the given message to bytes.
    fn serialize(&self, value: &Message) -> Result<Vec<u8>>;

    /// Deserializes bytes to a message.
    fn deserialize(&self, bytes: &[u8]) -> Result<Message>;
}

/// Creates a new [`Serializer`] for the given type.
pub fn new_serializer(serializer_type: SerializerType) -> Box<dyn Serializer> {
    match serializer_type {
        SerializerType::Json => Box::new(JsonSerializer::default()),
        SerializerType::MessagePack => Box::new(MessagePackSerializer::default()),
    }
}
