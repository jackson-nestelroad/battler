use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Item flags, which categorize items for miscellaneous behavior (such as bans or side
/// effects).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum ItemFlags {
    /// A ball.
    #[string = "Ball"]
    Ball,
    /// A berry.
    #[string = "Berry"]
    Berry,
    /// Locks the holder's move choice.
    #[string = "ChoiceLocking"]
    ChoiceLocking,
    /// Raises the user's evasion.
    #[string = "EvasionRaising"]
    EvasionRaising,
    ///A gem.
    #[string = "Gem"]
    Gem,
}

#[cfg(test)]
mod item_flags_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        items::ItemFlags,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(ItemFlags::EvasionRaising, "EvasionRaising");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("evasionraising", ItemFlags::EvasionRaising);
    }
}
