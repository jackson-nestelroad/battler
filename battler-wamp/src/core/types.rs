use serde::{
    Deserialize,
    Serialize,
};

use crate::core::hash::HashMap;

pub type Integer = u64;

pub type Dictionary = HashMap<String, Value>;

pub type List = Vec<Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Integer(Integer),
    String(String),
    Bool(bool),
    Dictionary(Dictionary),
    List(List),
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
