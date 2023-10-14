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
    #[string = "Legendary"]
    Legendary,
    #[string = "Mythical"]
    Mythical,
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
        test_string_serialization(SpeciesFlags::Legendary, "Legendary");
        test_string_serialization(SpeciesFlags::Mythical, "Mythical");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("legendary", SpeciesFlags::Legendary);
        test_string_deserialization("mythical", SpeciesFlags::Mythical);
    }
}
