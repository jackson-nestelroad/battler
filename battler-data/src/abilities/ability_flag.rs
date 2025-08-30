use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Ability flags, which categorize abilities for miscellaneous behavior (such as bans or side
/// effects).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum AbilityFlag {
    /// The ability can be broken by Mold Breaker.
    #[string = "Breakable"]
    Breakable,
    /// Raises the user's evasion.
    #[string = "EvasionRaising"]
    EvasionRaising,
    /// Cannot be copied by Role Play.
    #[string = "NoRolePlay"]
    NoRolePlay,
    /// Cannot be swapped by Skill Swap.
    #[string = "NoSkillSwap"]
    NoSkillSwap,
    /// Cannot be copied by Trace.
    #[string = "NoTrace"]
    NoTrace,
    /// Cannot activate when transformed.
    #[string = "NoTransform"]
    NoTransform,
    /// Cannot be overwritten by Worry Seed.
    #[string = "NoWorrySeed"]
    NoWorrySeed,
    /// The ability is permanently applied to the Mon. It cannot be changed or suppressed.
    #[string = "Permanent"]
    Permanent,
}

#[cfg(test)]
mod ability_flag_test {
    use crate::{
        AbilityFlag,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(AbilityFlag::EvasionRaising, "EvasionRaising");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("evasionraising", AbilityFlag::EvasionRaising);
    }
}
