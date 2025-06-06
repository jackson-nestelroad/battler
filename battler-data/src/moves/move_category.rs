use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The category of a move.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum MoveCategory {
    #[string = "Physical"]
    #[default]
    Physical,
    #[string = "Special"]
    Special,
    #[string = "Status"]
    Status,
}

#[cfg(test)]
mod move_category_test {
    use crate::{
        moves::MoveCategory,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(MoveCategory::Physical, "Physical");
        test_string_serialization(MoveCategory::Special, "Special");
        test_string_serialization(MoveCategory::Status, "Status");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("physical", MoveCategory::Physical);
        test_string_deserialization("special", MoveCategory::Special);
        test_string_deserialization("status", MoveCategory::Status);
    }
}
