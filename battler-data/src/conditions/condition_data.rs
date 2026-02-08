use alloc::string::String;

use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
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
    /// Z-Power, which is applied to the user of a Z-Move.
    #[string = "ZPower"]
    #[alias = "Z-Power"]
    ZPower,
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
    pub condition: serde_json::Value,
}

#[cfg(test)]
mod condition_test {
    use crate::{
        conditions::ConditionType,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
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
