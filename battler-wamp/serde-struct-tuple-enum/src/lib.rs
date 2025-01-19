//! # serde-struct-tuple-enum
//!
//! **serde-struct-tuple-enum** is a utility crate, built initially for [`battler-wamp`](https://crates.io/crates/battler-wamp). It provides procedural macros to automatically derive [`serde`](https://serde.rs/)'s `Serialize` and `Deserialize` traits for enum types, where each variant of the enum is a struct that is encoded as a tuple of its fields (specifically using [`serde-struct-tuple`](https://crates.io/crates/serde-struct-tuple)).
//!
//! Enums are expected to consist only of unit variants, wrapping a struct that implements
//! `serde_struct_tuple::SerializeStructTuple` and `serde_struct_tuple::DeserializeStructTuple`
//! (likely by using the corresponding derive macros).
//!
//! Each enum variant gets a single "tag" that is encoded as the first element in the tuple. This
//! tag allows the enum variant to be selected for deserializing the rest of the tuple.
//!
//! This message format directly corresponds to how [WAMP](https://wamp-proto.org/spec.html) messages are encoded.
//!
//! ## Example
//! ```
//! use std::collections::BTreeMap;
//!
//! use serde_struct_tuple::{
//!     DeserializeStructTuple,
//!     SerializeStructTuple,
//! };
//! use serde_struct_tuple_enum::{
//!     DeserializeStructTupleEnum,
//!     SerializeStructTupleEnum,
//! };
//!
//! #[derive(Debug, Default, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
//! struct Foo {
//!     a: u64,
//!     b: bool,
//!     #[serde_struct_tuple(default, skip_serializing_if = Vec::is_empty)]
//!     c: Vec<u64>,
//! }
//!
//! #[derive(Debug, Default, PartialEq, Eq, SerializeStructTuple, DeserializeStructTuple)]
//! struct Bar {
//!     a: String,
//!     #[serde_struct_tuple(default, skip_serializing_if = BTreeMap::is_empty)]
//!     b: BTreeMap<u64, u64>,
//! }
//!
//! #[derive(Debug, PartialEq, Eq, SerializeStructTupleEnum, DeserializeStructTupleEnum)]
//! #[tag(u64)]
//! enum Message {
//!     #[tag = 1]
//!     Foo(Foo),
//!     #[tag = 2]
//!     Bar(Bar),
//! }
//!
//! fn main() {
//!     // Serialization.
//!     assert_eq!(
//!         serde_json::to_string(&Message::Foo(Foo {
//!             a: 123,
//!             b: true,
//!             c: Vec::from_iter([7, 8, 9]),
//!         }))
//!         .unwrap(),
//!         r#"[1,123,true,[7,8,9]]"#
//!     );
//!     assert_eq!(
//!         serde_json::to_string(&Message::Bar(Bar {
//!             a: "hello".to_owned(),
//!             b: BTreeMap::from_iter([(3, 6), (4, 8)]),
//!         }))
//!         .unwrap(),
//!         r#"[2,"hello",{"3":6,"4":8}]"#
//!     );
//!
//!     // Deserialization.
//!     assert_eq!(
//!         serde_json::from_str::<Message>(r#"[1,1000,false]"#).unwrap(),
//!         Message::Foo(Foo {
//!             a: 1000,
//!             b: false,
//!             ..Default::default()
//!         })
//!     );
//!     assert_eq!(
//!         serde_json::from_str::<Message>(r#"[2,"goodbye",{"1":2,"3":4}]"#).unwrap(),
//!         Message::Bar(Bar {
//!             a: "goodbye".to_owned(),
//!             b: BTreeMap::from_iter([(1, 2), (3, 4)]),
//!         })
//!     );
//!     assert!(
//!         serde_json::from_str::<Message>(r#"[3]"#)
//!             .unwrap_err()
//!             .to_string()
//!             .contains("expected Message tuple")
//!     );
//! }
//! ```

pub use serde_struct_tuple_enum_proc_macro::{
    DeserializeStructTupleEnum,
    SerializeStructTupleEnum,
};
