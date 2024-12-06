use anyhow::{
    Error,
    Result,
};

use crate::{
    message::message::Message,
    serializer::serializer::Serializer,
};

/// A serializer implemented for JavaScript Object Notation.
#[derive(Default)]
pub struct JsonSerializer {}

impl Serializer for JsonSerializer {
    fn serialize(&self, value: &Message) -> Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(Error::new)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<Message> {
        Ok(serde_json::from_slice(bytes).map_err(Error::new)?)
    }
}
