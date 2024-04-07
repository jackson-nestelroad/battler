use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Species flags, which categorize species for miscellaneous behavior (such as bans or side
/// effects).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum SpeciesFlags {
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
}

#[cfg(test)]
mod species_flags_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        mons::SpeciesFlags,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(SpeciesFlags::RestrictedLegendary, "RestrictedLegendary");
        test_string_serialization(SpeciesFlags::Mythical, "Mythical");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("restricted legendary", SpeciesFlags::RestrictedLegendary);
        test_string_deserialization("mythical", SpeciesFlags::Mythical);
    }
}
