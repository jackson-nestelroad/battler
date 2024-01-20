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
    /// A condition that is built into the battle engine.
    #[string = "Built-in"]
    #[alias = "BuiltIn"]
    BuiltIn,
    /// An ordinary condition that can be applied to anything in a battle.
    #[string = "Condition"]
    Condition,
    /// Weather, which is applied to the entire battlefield.
    #[string = "Weather"]
    Weather,
    /// Status, which is applied to a single Mon for a finite amount of time.
    #[string = "Status"]
    Status,
}

/// Data about a particular condition.
///
/// Conditions can be applied to Mons as the result of moves or abilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionData {
    /// Condition name.
    pub name: String,
    /// Condition type.
    pub condition_type: ConditionType,
    /// Static duration for the condition.
    pub duration: Option<u8>,
    /// Can this condition be copied from one Mon to another?
    ///
    /// This relates to how "Baton Pass" affects this condition.
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
