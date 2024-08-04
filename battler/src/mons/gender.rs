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
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum Gender {
    #[string = "U"]
    #[alias = "Unknown"]
    Unknown,
    #[string = "F"]
    #[alias = "Female"]
    #[default]
    Female,
    #[string = "M"]
    #[alias = "Male"]
    Male,
}

#[cfg(test)]
mod gender_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        mons::Gender,
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
