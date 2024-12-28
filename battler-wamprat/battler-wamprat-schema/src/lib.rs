pub use battler_wamp::core::types::{
    Dictionary,
    Integer,
    List,
    Value,
};
pub use battler_wamprat_schema_proc_macro::{
    WampApplicationMessage,
    WampDictionary,
    WampList,
};
use thiserror::Error;

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

/// Trait for a WAMP application message, which can be passed between applications using pub/sub or
/// RPCs.
pub trait WampApplicationMessage: Sized {
    /// Serializes the object into arguments and keyword arguments.
    fn wamp_serialize_application_message(self) -> Result<(List, Dictionary), WampSerializeError>;

    /// Deserializes the object from arguments and keyword arguments.
    fn wamp_deserialize_application_message(
        arguments: List,
        arguments_keyword: Dictionary,
    ) -> Result<Self, WampDeserializeError>;
}
