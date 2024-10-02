use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The Mon to use for stat calculations on a move.
#[derive(Debug, Clone, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum MonOverride {
    /// Use the target for stat calculations.
    #[string = "Target"]
    Target,
    /// Use the user for stat calculations.
    #[string = "User"]
    User,
}

#[cfg(test)]
mod mon_override_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        moves::MonOverride,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(MonOverride::Target, "Target");
        test_string_serialization(MonOverride::User, "User");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("target", MonOverride::Target);
        test_string_deserialization("user", MonOverride::User);
    }
}
