use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::mons::Stat;

/// A Mon's nature, which boosts and drops particular stat values.
#[derive(
    Debug, Clone, Copy, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum Nature {
    #[string = "Hardy"]
    Hardy,
    #[string = "Lonely"]
    Lonely,
    #[string = "Adamant"]
    Adamant,
    #[string = "Naughty"]
    Naughty,
    #[string = "Brave"]
    Brave,
    #[string = "Bold"]
    Bold,
    #[string = "Docile"]
    Docile,
    #[string = "Impish"]
    Impish,
    #[string = "Lax"]
    Lax,
    #[string = "Relaxed"]
    Relaxed,
    #[string = "Modest"]
    Modest,
    #[string = "Mild"]
    Mild,
    #[string = "Bashful"]
    Bashful,
    #[string = "Rash"]
    Rash,
    #[string = "Quiet"]
    Quiet,
    #[string = "Calm"]
    Calm,
    #[string = "Gentle"]
    Gentle,
    #[string = "Careful"]
    Careful,
    #[string = "Quirky"]
    Quirky,
    #[string = "Sassy"]
    Sassy,
    #[string = "Timid"]
    Timid,
    #[string = "Hasty"]
    Hasty,
    #[string = "Jolly"]
    Jolly,
    #[string = "Naive"]
    Naive,
    #[string = "Serious"]
    Serious,
}

impl Nature {
    /// The stat boosted by the nature.
    pub fn boosts(&self) -> Stat {
        match self {
            Self::Hardy | Self::Lonely | Self::Adamant | Self::Naughty | Self::Brave => Stat::Atk,
            Self::Bold | Self::Docile | Self::Impish | Self::Lax | Self::Relaxed => Stat::Def,
            Self::Modest | Self::Mild | Self::Bashful | Self::Rash | Self::Quiet => Stat::SpAtk,
            Self::Calm | Self::Gentle | Self::Careful | Self::Quirky | Self::Sassy => Stat::SpDef,
            Self::Timid | Self::Hasty | Self::Jolly | Self::Naive | Self::Serious => Stat::Spe,
        }
    }

    /// The stat dopped by the nature.
    pub fn drops(&self) -> Stat {
        match self {
            Self::Hardy | Self::Bold | Self::Modest | Self::Calm | Self::Timid => Stat::Atk,
            Self::Lonely | Self::Docile | Self::Mild | Self::Gentle | Self::Hasty => Stat::Def,
            Self::Adamant | Self::Impish | Self::Bashful | Self::Careful | Self::Jolly => {
                Stat::SpAtk
            }
            Self::Naughty | Self::Lax | Self::Rash | Self::Quirky | Self::Naive => Stat::SpDef,
            Self::Brave | Self::Relaxed | Self::Quiet | Self::Sassy | Self::Serious => Stat::Spe,
        }
    }
}

#[cfg(test)]
mod nature_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        mons::Nature,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(Nature::Hardy, "Hardy");
        test_string_serialization(Nature::Lonely, "Lonely");
        test_string_serialization(Nature::Adamant, "Adamant");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("naughty", Nature::Naughty);
        test_string_deserialization("brave", Nature::Brave);
        test_string_deserialization("bold", Nature::Bold);
    }
}
