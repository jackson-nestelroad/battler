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
