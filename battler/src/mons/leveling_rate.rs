use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Leveling rate, which determines how much experience is required for a species to level up.
#[derive(Debug, Clone, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum LevelingRate {
    #[string = "Erratic"]
    Erratic,
    #[string = "Fast"]
    Fast,
    #[string = "Medium Fast"]
    #[alias = "Medium"]
    MediumFast,
    #[string = "Medium Slow"]
    MediumSlow,
    #[string = "Slow"]
    Slow,
    #[string = "Fluctuating"]
    Fluctuating,
}

#[cfg(test)]
mod leveling_rate_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        mons::LevelingRate,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(LevelingRate::Erratic, "Erratic");
        test_string_serialization(LevelingRate::Fast, "Fast");
        test_string_serialization(LevelingRate::MediumFast, "Medium Fast");
        test_string_serialization(LevelingRate::MediumSlow, "Medium Slow");
        test_string_serialization(LevelingRate::Slow, "Slow");
        test_string_serialization(LevelingRate::Fluctuating, "Fluctuating");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("erratic", LevelingRate::Erratic);
        test_string_deserialization("fast", LevelingRate::Fast);
        test_string_deserialization("medium fast", LevelingRate::MediumFast);
        test_string_deserialization("medium slow", LevelingRate::MediumSlow);
        test_string_deserialization("slow", LevelingRate::Slow);
        test_string_deserialization("fluctuating", LevelingRate::Fluctuating);
    }
}
