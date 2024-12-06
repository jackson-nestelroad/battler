pub use serde_struct_tuple_proc_macro::{
    DeserializeStructTuple,
    SerializeStructTuple,
};

/// Trait for deserializing a struct from a tuple of its fields.
pub trait DeserializeStructTuple {
    type Value;

    /// The [`serde::de::Visitor`] implementation that reads all fields from a sequence into the
    /// struct.
    fn visitor<'de>() -> impl serde::de::Visitor<'de, Value = Self::Value>;
}

/// Trait for serializing a struct into a tuple of its fields.
pub trait SerializeStructTuple {
    /// Serializes all struct fields to the given [`serde::ser::SerializeSeq`], in declaration
    /// order.
    fn serialize_fields_to_seq<S>(&self, seq: &mut S) -> core::result::Result<(), S::Error>
    where
        S: serde::ser::SerializeSeq;
}
