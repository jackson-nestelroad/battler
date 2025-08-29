use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Species flags, which categorize species for miscellaneous behavior (such as bans or side
/// effects).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum SpeciesFlag {
    #[string = "SubLegendary"]
    #[alias = "Sub-Legendary"]
    SubLegendary,
    #[string = "RestrictedLegendary"]
    #[alias = "Restricted Legendary"]
    RestrictedLegendary,
    #[string = "Mythical"]
    Mythical,
    #[string = "Paradox"]
    Paradox,
    #[string = "UltraBeast"]
    #[alias = "Ultra Beast"]
    UltraBeast,
}

#[cfg(test)]
mod species_flag_test {
    use crate::{
        SpeciesFlag,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(SpeciesFlag::RestrictedLegendary, "RestrictedLegendary");
        test_string_serialization(SpeciesFlag::Mythical, "Mythical");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("restricted legendary", SpeciesFlag::RestrictedLegendary);
        test_string_deserialization("mythical", SpeciesFlag::Mythical);
    }
}
