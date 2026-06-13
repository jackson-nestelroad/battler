use alloc::string::String;

use battler_data::{
    Fraction,
    Id,
    MoveData,
};
use hashbrown::HashMap;
use serde::{
    Deserialize,
    Serialize,
};

use crate::effect::fxlang::Value;

/// A custom value in [`LocalData`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalDataValue {
    String(String),
    Number(Fraction<i64>),
}

impl Into<Value> for LocalDataValue {
    fn into(self) -> Value {
        match self {
            Self::String(val) => Value::String(val),
            Self::Number(val) => Value::Fraction(val),
        }
    }
}

/// Local data to an fxlang effect or condition.
///
/// Data here can be referenced by callbacks in the owning effect or condition.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LocalData {
    /// Custom moves that can be used by the effect.
    #[serde(default)]
    pub moves: HashMap<Id, MoveData>,

    /// Values that can be referenced by effect callbacks.
    #[serde(default)]
    pub values: HashMap<String, LocalDataValue>,
}

impl LocalData {
    /// Extends the local data with other local data, overriding data if applicable.
    pub fn extend(&mut self, other: Self) {
        self.moves.extend(other.moves);
        self.values.extend(other.values);
    }
}
