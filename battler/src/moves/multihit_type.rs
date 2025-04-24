use std::{
    fmt,
    fmt::Display,
};

use anyhow::Error;
use serde::{
    de::Visitor,
    ser::SerializeSeq,
    Deserialize,
    Serialize,
    Serializer,
};

use crate::error::general_error;

/// The number of hits done by a multihit move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultihitType {
    /// A static number of hits.
    Static(u8),
    /// A range of numbers to choose from.
    Range(u8, u8),
}

impl Display for MultihitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(n) => write!(f, "{n}"),
            Self::Range(begin, end) => write!(f, "[{begin},{end}]"),
        }
    }
}

impl From<u8> for MultihitType {
    fn from(value: u8) -> Self {
        Self::Static(value)
    }
}

impl TryFrom<&[u8]> for MultihitType {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(general_error(
                "multihit range must contain exactly 2 elements",
            ));
        }
        Ok(Self::Range(value[0], value[1]))
    }
}

impl Serialize for MultihitType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Static(n) => serializer.serialize_u8(*n),
            Self::Range(begin, end) => {
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(begin)?;
                seq.serialize_element(end)?;
                seq.end()
            }
        }
    }
}

struct MultihitTypeVisitor;

impl<'de> Visitor<'de> for MultihitTypeVisitor {
    type Value = MultihitType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an integer or an array of 2 integers")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Self::Value::from(v as u8))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let begin = match seq.next_element()? {
            Some(v) => v,
            None => return Err(serde::de::Error::invalid_length(0, &self)),
        };
        let end = match seq.next_element()? {
            Some(v) => v,
            None => return Err(serde::de::Error::invalid_length(1, &self)),
        };
        if seq.next_element::<u8>()?.is_some() {
            return Err(serde::de::Error::invalid_length(3, &self));
        }
        Ok(Self::Value::Range(begin, end))
    }
}

impl<'de> Deserialize<'de> for MultihitType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(MultihitTypeVisitor)
    }
}

#[cfg(test)]
mod multihit_type_tests {
    use crate::{
        common::test_serialization,
        moves::MultihitType,
    };

    #[test]
    fn serializes_to_string() {
        test_serialization(MultihitType::Static(2), 2);
        test_serialization(MultihitType::Range(1, 5), "[1,5]");
    }
}
