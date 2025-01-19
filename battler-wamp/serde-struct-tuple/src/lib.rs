//! # serde-struct-tuple
//!
//! **serde-struct-tuple** is a utility crate, built initially for [`battler-wamp`](https://crates.io/crates/battler-wamp). It provides procedural macros to automatically derive [`serde`](https://serde.rs/)'s `Serialize` and `Deserialize` traits for struct types that should be encoded as a tuple (list) of its fields.
//!
//! Struct fields can be any type that implement `serde::Serialize` and/or `serde::Deserialize`.
//!
//! The macro also has additional optional attributes for struct fields:
//!
//! * `default` - If the field is missing during deserialization, the field is initialized to its
//!   default value.
//! * `skip_serializing_if` - Checks if the field should be skipped during serialization using the
//!   function provided. All subsequent fields will also be skipped, regardless of their value.
//!
//! ## Example
//!
//! ```
//! use std::collections::BTreeMap;
//!
//! use serde_struct_tuple::{
//!     DeserializeStructTuple,
//!     SerializeStructTuple,
//! };
//!
//! fn is_true(b: &bool) -> bool {
//!     *b
//! }
//!
//! #[derive(Debug, Default, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
//! struct Message {
//!     a: u64,
//!     b: String,
//!     #[serde_struct_tuple(default, skip_serializing_if = Vec::is_empty)]
//!     c: Vec<u64>,
//!     #[serde_struct_tuple(default, skip_serializing_if = BTreeMap::is_empty)]
//!     d: BTreeMap<u8, bool>,
//!     #[serde_struct_tuple(default, skip_serializing_if = is_true)]
//!     e: bool,
//! }
//!
//! fn main() {
//!     // Serialization.
//!     assert_eq!(
//!         serde_json::to_string(&Message {
//!             a: 123,
//!             b: "foo".to_owned(),
//!             ..Default::default()
//!         })
//!         .unwrap(),
//!         r#"[123,"foo"]"#
//!     );
//!     assert_eq!(
//!         serde_json::to_string(&Message {
//!             a: 123,
//!             b: "foo".to_owned(),
//!             // Skipped because `c` is skipped.
//!             d: BTreeMap::from_iter([(1, false), (2, true)]),
//!             ..Default::default()
//!         })
//!         .unwrap(),
//!         r#"[123,"foo"]"#
//!     );
//!     assert_eq!(
//!         serde_json::to_string(&Message {
//!             a: 123,
//!             b: "foo".to_owned(),
//!             c: Vec::from_iter([6, 7, 8]),
//!             d: BTreeMap::from_iter([(1, false), (2, true)]),
//!             ..Default::default()
//!         })
//!         .unwrap(),
//!         r#"[123,"foo",[6,7,8],{"1":false,"2":true},false]"#
//!     );
//!
//!     // Deserialization.
//!     assert_eq!(
//!         serde_json::from_str::<Message>(r#"[123, "foo"]"#).unwrap(),
//!         Message {
//!             a: 123,
//!             b: "foo".to_owned(),
//!             ..Default::default()
//!         }
//!     );
//!     assert_eq!(
//!         serde_json::from_str::<Message>(r#"[123, "foo", [99, 100], { "20": true }, true]"#)
//!             .unwrap(),
//!         Message {
//!             a: 123,
//!             b: "foo".to_owned(),
//!             c: Vec::from_iter([99, 100]),
//!             d: BTreeMap::from_iter([(20, true)]),
//!             e: true,
//!         }
//!     );
//! }
//! ```

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
