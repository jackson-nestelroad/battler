use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The gender of a specific Mon.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub enum Gender {
    #[string = "U"]
    #[alias = "Unknown"]
    #[default]
    #[cfg_attr(feature = "typescript", ts(rename = "U"))]
    Unknown,
    #[string = "F"]
    #[alias = "Female"]
    #[cfg_attr(feature = "typescript", ts(rename = "F"))]
    Female,
    #[string = "M"]
    #[alias = "Male"]
    #[cfg_attr(feature = "typescript", ts(rename = "M"))]
    Male,
}

#[cfg(test)]
mod gender_test {
    use crate::{
        mons::Gender,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(Gender::Unknown, "U");
        test_string_serialization(Gender::Female, "F");
        test_string_serialization(Gender::Male, "M");
    }

    #[test]
    fn deserializes_full_name() {
        test_string_deserialization("Unknown", Gender::Unknown);
        test_string_deserialization("Female", Gender::Female);
        test_string_deserialization("Male", Gender::Male);
    }
}
