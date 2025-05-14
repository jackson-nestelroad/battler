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
    #[default]
    Unknown,
    #[string = "F"]
    #[alias = "Female"]
    Female,
    #[string = "M"]
    #[alias = "Male"]
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
