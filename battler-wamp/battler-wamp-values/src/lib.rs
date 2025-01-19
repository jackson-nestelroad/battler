//! # battler-wamp-values
//!
//! **battler-wamp-values** is a utility crate for [`battler-wamp`](https://crates.io/crates/battler-wamp). It provides core value type definitions, as well as procedural macros for serializing Rust structs into WAMP lists and dictionaries.
//!
//! In WAMP, all parts of a message can be boiled into a [`Value`]. For a Rust type to be encoded
//! into a WAMP message, it must be convertible to a [`Value`]. This behavior is covered by the
//! [`WampSerialize`] and [`WampDeserialize`] traits.
//!
//! For convenience, the [`WampDictionary`] and [`WampList`] procedural macros can be used to
//! automatically derive the [`WampSerialize`] and [`WampDeserialize`] traits for complex Rust
//! types. Both of these macros assume that all struct fields also implement these traits.
//!
//! These macro also have additional optional attributes for struct fields:
//!
//! * `default` - If the field is missing during deserialization, the field is initialized to its
//!   default value.
//! * `skip_serializing_if` - Checks if the field should be skipped during serialization using the
//!   function provided. For lists, all subsequent fields will also be skipped, regardless of their
//!   value.
//!
//!
//! ## Example
//!
//! ```
//! use battler_wamp_values::{
//!     Dictionary,
//!     Integer,
//!     List,
//!     Value,
//!     WampDeserialize,
//!     WampDictionary,
//!     WampList,
//!     WampSerialize,
//! };
//!
//! #[derive(Debug, PartialEq, Eq, WampDictionary)]
//! struct Metadata {
//!     version: Integer,
//!     #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
//!     feature_enabled: Option<bool>,
//!     name: String,
//! }
//!
//! #[derive(Debug, PartialEq, Eq, WampList)]
//! struct Args(
//!     Integer,
//!     Integer,
//!     #[battler_wamp_values(default, skip_serializing_if = List::is_empty)] List,
//! );
//!
//! fn main() {
//!     // Serialization.
//!     assert_eq!(
//!         Metadata {
//!             version: 1,
//!             feature_enabled: None,
//!             name: "foo".to_owned(),
//!         }
//!         .wamp_serialize()
//!         .unwrap(),
//!         Value::Dictionary(Dictionary::from_iter([
//!             ("version".to_owned(), Value::Integer(1)),
//!             ("name".to_owned(), Value::String("foo".to_owned())),
//!         ]))
//!     );
//!     assert_eq!(
//!         Args(1, 2, Vec::from_iter((3..6).map(Value::Integer)))
//!             .wamp_serialize()
//!             .unwrap(),
//!         Value::List(List::from_iter([
//!             Value::Integer(1),
//!             Value::Integer(2),
//!             Value::List(List::from_iter([
//!                 Value::Integer(3),
//!                 Value::Integer(4),
//!                 Value::Integer(5),
//!             ])),
//!         ]))
//!     );
//!
//!     // Deserialization.
//!     assert_eq!(
//!         Metadata::wamp_deserialize(Value::Dictionary(Dictionary::from_iter([
//!             ("version".to_owned(), Value::Integer(2)),
//!             ("name".to_owned(), Value::String("bar".to_owned())),
//!             ("feature_enabled".to_owned(), Value::Bool(false)),
//!         ])))
//!         .unwrap(),
//!         Metadata {
//!             version: 2,
//!             name: "bar".to_owned(),
//!             feature_enabled: Some(false),
//!         }
//!     );
//!     assert_eq!(
//!         Args::wamp_deserialize(Value::List(List::from_iter([
//!             Value::Integer(7),
//!             Value::Integer(8),
//!         ])))
//!         .unwrap(),
//!         Args(7, 8, List::default())
//!     );
//! }
//! ```

pub use battler_wamp_values_proc_macro::{
    WampDictionary,
    WampList,
};
use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

/// An integer type for WAMP messages.
pub type Integer = u64;

/// A dictionary of key-value pairs.
pub type Dictionary = ahash::HashMap<String, Value>;

/// A sequence of values.
pub type List = Vec<Value>;

/// A value for WAMP messages.
///
/// In WAMP, all parts of a message can be boiled into a [`Value`]. The [`WampSerialize`] and
/// [`WampDeserialize`] traits handle the conversion of Rust types into WAMP values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Integer(Integer),
    String(String),
    Bool(bool),
    Dictionary(Dictionary),
    List(List),
}

impl Value {
    /// The value as an [`Integer`].
    pub fn integer(&self) -> Option<Integer> {
        match self {
            Self::Integer(val) => Some(*val),
            _ => None,
        }
    }

    /// The value as a [`str`].
    pub fn string(&self) -> Option<&str> {
        match self {
            Self::String(val) => Some(val),
            _ => None,
        }
    }

    /// The value as a [`bool`].
    pub fn bool(&self) -> Option<bool> {
        match self {
            Self::Bool(val) => Some(*val),
            _ => None,
        }
    }

    /// The value as a [`Dictionary`].
    pub fn dictionary(&self) -> Option<&Dictionary> {
        match self {
            Self::Dictionary(val) => Some(val),
            _ => None,
        }
    }

    /// The value as a [`Dictionary`].
    pub fn dictionary_mut(&mut self) -> Option<&mut Dictionary> {
        match self {
            Self::Dictionary(val) => Some(val),
            _ => None,
        }
    }

    /// The value as a [`List`].
    pub fn list(&self) -> Option<&List> {
        match self {
            Self::List(val) => Some(val),
            _ => None,
        }
    }

    /// The value as a [`List`].
    pub fn list_mut(&mut self) -> Option<&mut List> {
        match self {
            Self::List(val) => Some(val),
            _ => None,
        }
    }
}

impl From<Integer> for Value {
    fn from(value: Integer) -> Self {
        Self::Integer(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Dictionary> for Value {
    fn from(value: Dictionary) -> Self {
        Self::Dictionary(value)
    }
}

impl From<List> for Value {
    fn from(value: List) -> Self {
        Self::List(value)
    }
}

/// An error resulting from serializing a Rust object into a WAMP value using the [`WampSerialize`]
/// trait.
#[derive(Debug, Error)]
#[error("{msg}")]
pub struct WampSerializeError {
    msg: String,
}

impl WampSerializeError {
    pub fn new<S>(msg: S) -> Self
    where
        S: Into<String>,
    {
        Self { msg: msg.into() }
    }

    pub fn annotate(&self, msg: String) -> Self {
        Self::new(format!("{}; {msg}", self.msg))
    }
}

/// An error resulting from deserializing a Rust object from a WAMP value using the
/// [`WampDeserialize`] trait.
#[derive(Debug, Error)]
#[error("{msg}")]
pub struct WampDeserializeError {
    msg: String,
}

impl WampDeserializeError {
    pub fn new<S>(msg: S) -> Self
    where
        S: Into<String>,
    {
        Self { msg: msg.into() }
    }

    pub fn annotate(&self, msg: String) -> Self {
        Self::new(format!("{}; {msg}", self.msg))
    }
}

/// Trait for serializing a Rust object into a WAMP value.
pub trait WampSerialize {
    /// Serializes the object into a WAMP value.
    fn wamp_serialize(self) -> Result<Value, WampSerializeError>;
}

impl WampSerialize for Integer {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        Ok(Value::Integer(self))
    }
}

impl WampSerialize for String {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        Ok(Value::String(self))
    }
}

impl WampSerialize for bool {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        Ok(Value::Bool(self))
    }
}

impl WampSerialize for List {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        Ok(Value::List(self))
    }
}

impl WampSerialize for Dictionary {
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        Ok(Value::Dictionary(self))
    }
}

impl<T> WampSerialize for Option<T>
where
    T: WampSerialize,
{
    fn wamp_serialize(self) -> Result<Value, WampSerializeError> {
        match self {
            Some(val) => val.wamp_serialize(),
            None => Err(WampSerializeError::new(
                "empty optional cannot be serialized",
            )),
        }
    }
}

/// Trait for deserializing a Rust object from a WAMP value.
pub trait WampDeserialize: Sized {
    /// Deserializes the object from a WAMP value.
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError>;
}

impl WampDeserialize for Integer {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        match value {
            Value::Integer(val) => Ok(val),
            _ => Err(WampDeserializeError::new("value must be an integer")),
        }
    }
}

impl WampDeserialize for String {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        match value {
            Value::String(val) => Ok(val),
            _ => Err(WampDeserializeError::new("value must be a string")),
        }
    }
}

impl WampDeserialize for bool {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        match value {
            Value::Bool(val) => Ok(val),
            _ => Err(WampDeserializeError::new("value must be a bool")),
        }
    }
}

impl WampDeserialize for List {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        match value {
            Value::List(val) => Ok(val),
            _ => Err(WampDeserializeError::new("value must be a list")),
        }
    }
}

impl WampDeserialize for Dictionary {
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        match value {
            Value::Dictionary(val) => Ok(val),
            _ => Err(WampDeserializeError::new("value must be a dictionary")),
        }
    }
}

impl<T> WampDeserialize for Option<T>
where
    T: WampDeserialize,
{
    fn wamp_deserialize(value: Value) -> Result<Self, WampDeserializeError> {
        Ok(Some(T::wamp_deserialize(value)?))
    }
}
