use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;
use battler_wamp_values::{
    Integer,
    Value,
    WampDeserialize,
    WampDeserializeError,
    WampSerialize,
    WampSerializeError,
};
use futures_util::lock::Mutex;
use serde::{
    Deserialize,
    Serialize,
    de::{
        Unexpected,
        Visitor,
    },
};
use thiserror::Error;

/// An integer ID, used for identification of resources and requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Id(u64);

impl Id {
    /// The minimum allowable value of an ID.
    pub const MIN: Id = Id(1);

    /// The maximum allowable value of an ID.
    pub const MAX: Id = Id(1 << 53);
}

impl Default for Id {
    fn default() -> Self {
        Id::MIN
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl WampSerialize for Id {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        self.0.wamp_serialize()
    }
}

impl WampDeserialize for Id {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        Id::try_from(Integer::wamp_deserialize(value)?)
            .map_err(|_| WampDeserializeError::new("invalid id"))
    }
}

/// Error for an ID being out of range.
#[derive(Debug, Error)]
#[error("{value} is out of range for IDs")]
pub struct IdOutOfRange {
    value: u64,
}

impl IdOutOfRange {
    fn new(value: u64) -> Self {
        Self { value }
    }
}

impl TryFrom<u64> for Id {
    type Error = IdOutOfRange;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value < Self::MIN.0 || value > Self::MAX.0 {
            Err(IdOutOfRange::new(value))
        } else {
            Ok(Id(value))
        }
    }
}

struct IdVisitor;

impl<'de> Visitor<'de> for IdVisitor {
    type Value = Id;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "an unsigned integer in the range [{}, {}]",
            Id::MIN,
            Id::MAX
        )
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Id::try_from(v).map_err(|_| E::invalid_value(Unexpected::Unsigned(v), &self))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_u64(IdVisitor)
    }
}

/// An ID allocator.
#[async_trait]
pub trait IdAllocator: Send + Sync {
    /// Generates a new ID.
    async fn generate_id(&self) -> Id;

    /// Resets the allocator to its initial state.
    async fn reset(&self);
}

/// An ID allocator that generates IDs from a random sequence.
///
/// Used for global-scoped IDs.
#[derive(Debug, Default)]
pub struct RandomIdAllocator {}

#[async_trait]
impl IdAllocator for RandomIdAllocator {
    async fn generate_id(&self) -> Id {
        let id = (rand::random::<u64>() & (Id::MAX.0 - 1)) + 1;
        Id(id)
    }

    async fn reset(&self) {}
}

/// An ID allocator that generates IDs sequentially.
///
/// Used for session-scoped IDs.
#[derive(Debug, Default)]
pub struct SequentialIdAllocator {
    next: Mutex<Id>,
}

#[async_trait]
impl IdAllocator for SequentialIdAllocator {
    async fn generate_id(&self) -> Id {
        let mut lock = self.next.lock().await;
        let id = *lock;
        let next = if id.0 == Id::MAX.0 { 1 } else { id.0 + 1 };
        let next = Id::try_from(next).unwrap();
        *lock = next;
        id
    }

    async fn reset(&self) {
        let mut lock = self.next.lock().await;
        *lock = Id::MIN;
    }
}

#[cfg(test)]
mod id_test {
    use crate::core::id::Id;

    #[test]
    fn fails_deserialization_out_of_range() {
        assert_matches::assert_matches!(serde_json::from_str::<Id>(r#"0"#), Err(err) => {
            assert!(err.to_string().contains("expected an unsigned integer in the range"));
        });
        assert_matches::assert_matches!(serde_json::from_str::<Id>(r#"9007199254740993"#), Err(err) => {
            assert!(err.to_string().contains("expected an unsigned integer in the range"));
        });
    }
}
