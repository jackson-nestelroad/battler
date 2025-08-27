use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Item flags, which categorize items for miscellaneous behavior (such as bans or side
/// effects).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum ItemFlag {
    /// A ball.
    #[string = "Ball"]
    Ball,
    /// A battle item.
    #[string = "Battle"]
    Battle,
    /// A berry.
    #[string = "Berry"]
    Berry,
    /// Locks the holder's move choice.
    #[string = "ChoiceLocking"]
    ChoiceLocking,
    /// A damage-reducing berry.
    #[string = "DamageReducingBerry"]
    DamageReducingBerry,
    /// Raises the user's evasion.
    #[string = "EvasionRaising"]
    EvasionRaising,
    /// A gem.
    #[string = "Gem"]
    Gem,
    /// Medicine.
    #[string = "Medicine"]
    Medicine,
    /// Not affected by Klutz.
    #[string = "NoKlutz"]
    NoKlutz,
}

#[cfg(test)]
mod item_flag_test {
    use crate::{
        ItemFlag,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(ItemFlag::EvasionRaising, "EvasionRaising");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("evasionraising", ItemFlag::EvasionRaising);
    }
}
