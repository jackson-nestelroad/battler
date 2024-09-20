use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The acceptable target of an item.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum ItemInput {
    /// Move slot index.
    #[string = "MoveSlot"]
    MoveSlot,
}

#[cfg(test)]
mod item_input_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        items::ItemInput,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(ItemInput::MoveSlot, "MoveSlot");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("moveslot", ItemInput::MoveSlot);
    }
}
