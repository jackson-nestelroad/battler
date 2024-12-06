use anyhow::{
    Error,
    Result,
};

use crate::{
    message::message::Message,
    serializer::serializer::Serializer,
};

/// A serializer implemented for MessagePack.
#[derive(Default)]
pub struct MessagePackSerializer {}

impl Serializer for MessagePackSerializer {
    fn serialize(&self, value: &Message) -> Result<Vec<u8>> {
        rmp_serde::to_vec(value).map_err(Error::new)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<Message> {
        Ok(rmp_serde::from_slice(bytes).map_err(Error::new)?)
    }
}
