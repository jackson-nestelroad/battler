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
mod item_input_test {
    use crate::{
        items::ItemInput,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
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
