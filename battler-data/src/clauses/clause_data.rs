use alloc::string::String;

use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::SerializedRuleSet;

/// The type of a clause value.
#[derive(Debug, Clone, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum ClauseValueType {
    #[string = "Type"]
    Type,
    #[string = "PositiveInteger"]
    PositiveInteger,
    #[string = "NonNegativeInteger"]
    NonNegativeInteger,
}

/// Data for an individual clause.
///
/// A clause is a generalization of a rule: a clause can be a compound rule made up of several more
/// rules, or it can be a simple rule with an assigned value.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClauseData {
    /// Clause name.
    pub name: String,
    /// Clause description.
    pub description: String,
    /// Message added to the battle log on battle start.
    #[serde(default)]
    pub rule_log: Option<String>,
    /// Is a value required?
    #[serde(default)]
    pub requires_value: bool,
    /// Type of value enforced by validation.
    #[serde(default)]
    pub value_type: Option<ClauseValueType>,
    /// The default value.
    #[serde(default)]
    pub default_value: String,
    /// Nested rules added to the battle format.
    #[serde(default)]
    pub rules: SerializedRuleSet,

    /// Dynamic battle effects.
    #[serde(default)]
    pub effect: serde_json::Value,
}

#[cfg(test)]
mod clause_value_type_test {
    use crate::{
        ClauseValueType,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(ClauseValueType::Type, "Type");
        test_string_serialization(ClauseValueType::PositiveInteger, "PositiveInteger");
        test_string_serialization(ClauseValueType::NonNegativeInteger, "NonNegativeInteger");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("type", ClauseValueType::Type);
        test_string_deserialization("positiveinteger", ClauseValueType::PositiveInteger);
        test_string_deserialization("nonnegativeinteger", ClauseValueType::NonNegativeInteger);
    }
}
