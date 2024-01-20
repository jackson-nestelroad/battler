use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::common::{
    Id,
    Identifiable,
};

/// The type of a condition.
#[derive(Debug, Clone, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum ConditionType {
    #[string = "Condition"]
    Condition,
    #[string = "Weather"]
    Weather,
    #[string = "Status"]
    Status,
}

/// Data about a particular condition.
///
/// Conditions can be applied to Mons as the result of moves or abilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionData {
    pub name: String,
    pub condition_type: ConditionType,
    pub duration: Option<u8>,
    #[serde(default)]
    pub no_copy: bool,
}

/// An individual condition, which can affect a Mon in a variety of ways.
pub struct Condition {
    pub data: ConditionData,
    id: Id,
}

impl Condition {
    pub fn new(data: ConditionData) -> Self {
        let id = Id::from(data.name.as_ref());
        Self { data, id }
    }
}

impl Identifiable for Condition {
    fn id(&self) -> &Id {
        &self.id
    }
}
