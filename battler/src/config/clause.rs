use std::str::FromStr;

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
    config::SerializedRuleSet,
    effect::fxlang,
    error::{
        general_error,
        Error,
    },
    mons::Type,
};

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
    pub effect: fxlang::Effect,
}

/// A rule that modifies the validation, start, or team preview stages of a battle.
///
/// A clause is a generalization of a rule: a clause can be a compound rule made up of several more
/// rules, or it can be a simple rule with an assigned value.
#[derive(Clone)]
pub struct Clause {
    id: Id,
    pub data: ClauseData,
}

impl Clause {
    /// Creates a new clause.
    pub fn new(id: Id, data: ClauseData) -> Self {
        Self { id, data }
    }

    /// Validates the given value according to clause's configuration.
    pub fn validate_value(&self, value: &str) -> Result<(), Error> {
        if value.is_empty() {
            if self.data.requires_value {
                return Err(general_error("missing value"));
            }
            Ok(())
        } else {
            match self.data.value_type {
                Some(ClauseValueType::Type) => {
                    Type::from_str(value).map_err(general_error).map(|_| ())
                }
                Some(ClauseValueType::PositiveInteger) => {
                    value.parse::<u32>().map_err(general_error).and_then(|val| {
                        if val > 0 {
                            Ok(())
                        } else {
                            Err(general_error("integer cannot be 0"))
                        }
                    })
                }
                Some(ClauseValueType::NonNegativeInteger) => {
                    value.parse::<u32>().map_err(general_error).map(|_| ())
                }
                _ => Ok(()),
            }
        }
    }
}

impl Identifiable for Clause {
    fn id(&self) -> &Id {
        &self.id
    }
}

#[cfg(test)]
mod clause_value_type_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        config::ClauseValueType,
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
