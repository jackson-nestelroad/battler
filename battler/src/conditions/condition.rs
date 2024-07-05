use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::{
    common::{
        Id,
        Identifiable,
    },
    effect::fxlang,
};

/// The type of a condition.
#[derive(Debug, Clone, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
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
    /// Type, which is applied to a single Mon for a finite amount of time while the user has the
    /// type.
    #[string = "Type"]
    Type,
    /// Volatile, which is applied to a single Mon for a finite amount of time.
    #[string = "Volatile"]
    Volatile,
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
    /// Can this condition be copied from one Mon to another?
    ///
    /// This relates to how "Baton Pass" affects this condition.
    #[serde(default)]
    pub no_copy: bool,

    /// Dynamic battle effects.
    #[serde(default)]
    pub condition: fxlang::Condition,
}

/// An individual condition, which can affect a Mon in a variety of ways.
pub struct Condition {
    id: Id,
    pub data: ConditionData,
}

impl Condition {
    pub fn new(id: Id, data: ConditionData) -> Self {
        Self { id, data }
    }

    pub fn condition_type_name(&self) -> &str {
        match self.data.condition_type {
            ConditionType::BuiltIn => "",
            ConditionType::Condition => "condition",
            ConditionType::Status => "status",
            ConditionType::Type => "type",
            ConditionType::Volatile => "",
            ConditionType::Weather => "weather",
        }
    }

    pub fn non_empty_condition_type_name(&self) -> &str {
        let type_name = self.condition_type_name();
        if type_name.is_empty() {
            "condition"
        } else {
            type_name
        }
    }
}

impl Identifiable for Condition {
    fn id(&self) -> &Id {
        &self.id
    }
}

#[cfg(test)]
mod condition_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        conditions::ConditionType,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(ConditionType::BuiltIn, "Built-in");
        test_string_serialization(ConditionType::Condition, "Condition");
        test_string_serialization(ConditionType::Weather, "Weather");
        test_string_serialization(ConditionType::Status, "Status");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("built-in", ConditionType::BuiltIn);
        test_string_deserialization("builtin", ConditionType::BuiltIn);
        test_string_deserialization("condition", ConditionType::Condition);
        test_string_deserialization("weather", ConditionType::Weather);
        test_string_deserialization("status", ConditionType::Status);
    }
}
