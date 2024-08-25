use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Ability flags, which categorize abilities for miscellaneous behavior (such as bans or side
/// effects).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum AbilityFlags {
    /// The ability can be broken by Mold Breaker.
    #[string = "Breakable"]
    Breakable,
    /// Raises the user's evasion.
    #[string = "EvasionRaising"]
    EvasionRaising,
    /// Cannot be copied by Role Play.
    #[string = "NoRolePlay"]
    NoRolePlay,
    /// The ability is permanently applied to the Mon.
    #[string = "Permanent"]
    Permanent,
}

#[cfg(test)]
mod ability_flags_tests {
    use crate::{
        abilities::AbilityFlags,
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(AbilityFlags::EvasionRaising, "EvasionRaising");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("evasionraising", AbilityFlags::EvasionRaising);
    }
}
