use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Leveling rate, which determines how much experience is required for a species to level up.
#[derive(
    Debug, Clone, Copy, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
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

impl LevelingRate {
    /// The amount of experience a Mon at the given level should have.
    pub fn exp_at_level(&self, level: u8) -> u32 {
        if level == 1 {
            return 0;
        }
        let level = level as u32;
        let squared_level = level * level;
        let cubed_level = level * level * level;
        match self {
            Self::Erratic => {
                if level < 50 {
                    (cubed_level * (100 - level)) / 50
                } else if level < 68 {
                    (cubed_level * (150 - level)) / 100
                } else if level < 98 {
                    (cubed_level * ((1911 - 10 * level) / 3)) / 500
                } else {
                    (cubed_level * (160 - level)) / 100
                }
            }
            Self::Fast => (4 * cubed_level) / 5,
            Self::MediumFast => cubed_level,
            Self::MediumSlow => {
                ((6 * cubed_level) / 5)
                    .overflowing_sub(15 * squared_level)
                    .0
                    .overflowing_add(100 * level)
                    .0
                    - 140
            }
            Self::Slow => (5 * cubed_level) / 4,
            Self::Fluctuating => {
                if level < 15 {
                    (cubed_level * ((level + 1) / 3 + 24)) / 50
                } else if level < 36 {
                    (cubed_level * (level + 14)) / 50
                } else {
                    (cubed_level * (level / 2 + 32)) / 50
                }
            }
        }
    }

    /// Calculates a Mon's level based on experience points.
    pub fn level_from_exp(&self, exp: u32) -> u8 {
        let mut min = 1;
        let mut max = 100;
        while max - min > 1 {
            let mid = (max - min) / 2 + min;
            let mid_exp = self.exp_at_level(mid);
            if exp < mid_exp {
                max = mid - 1;
            } else if exp > mid_exp {
                // We can be between levels.
                min = mid;
            } else {
                min = mid;
                max = mid;
            }
        }
        if exp >= self.exp_at_level(max) {
            max
        } else {
            min
        }
    }
}

#[cfg(test)]
mod leveling_rate_test {
    use crate::{
        mons::LevelingRate,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
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

    #[test]
    fn calculates_experience_at_level() {
        assert_eq!(LevelingRate::Erratic.exp_at_level(1), 0);
        assert_eq!(LevelingRate::Erratic.exp_at_level(2), 15);
        assert_eq!(LevelingRate::Erratic.exp_at_level(20), 12800);
        assert_eq!(LevelingRate::Erratic.exp_at_level(50), 125000);
        assert_eq!(LevelingRate::Erratic.exp_at_level(77), 346965);
        assert_eq!(LevelingRate::Erratic.exp_at_level(100), 600000);

        assert_eq!(LevelingRate::Fast.exp_at_level(1), 0);
        assert_eq!(LevelingRate::Fast.exp_at_level(2), 6);
        assert_eq!(LevelingRate::Fast.exp_at_level(20), 6400);
        assert_eq!(LevelingRate::Fast.exp_at_level(50), 100000);
        assert_eq!(LevelingRate::Fast.exp_at_level(77), 365226);
        assert_eq!(LevelingRate::Fast.exp_at_level(100), 800000);

        assert_eq!(LevelingRate::MediumFast.exp_at_level(1), 0);
        assert_eq!(LevelingRate::MediumFast.exp_at_level(2), 8);
        assert_eq!(LevelingRate::MediumFast.exp_at_level(20), 8000);
        assert_eq!(LevelingRate::MediumFast.exp_at_level(50), 125000);
        assert_eq!(LevelingRate::MediumFast.exp_at_level(77), 456533);
        assert_eq!(LevelingRate::MediumFast.exp_at_level(100), 1000000);

        assert_eq!(LevelingRate::MediumSlow.exp_at_level(1), 0);
        assert_eq!(LevelingRate::MediumSlow.exp_at_level(2), 9);
        assert_eq!(LevelingRate::MediumSlow.exp_at_level(20), 5460);
        assert_eq!(LevelingRate::MediumSlow.exp_at_level(50), 117360);
        assert_eq!(LevelingRate::MediumSlow.exp_at_level(77), 466464);
        assert_eq!(LevelingRate::MediumSlow.exp_at_level(100), 1059860);

        assert_eq!(LevelingRate::Slow.exp_at_level(1), 0);
        assert_eq!(LevelingRate::Slow.exp_at_level(2), 10);
        assert_eq!(LevelingRate::Slow.exp_at_level(20), 10000);
        assert_eq!(LevelingRate::Slow.exp_at_level(50), 156250);
        assert_eq!(LevelingRate::Slow.exp_at_level(77), 570666);
        assert_eq!(LevelingRate::Slow.exp_at_level(100), 1250000);

        assert_eq!(LevelingRate::Fluctuating.exp_at_level(1), 0);
        assert_eq!(LevelingRate::Fluctuating.exp_at_level(2), 4);
        assert_eq!(LevelingRate::Fluctuating.exp_at_level(20), 5440);
        assert_eq!(LevelingRate::Fluctuating.exp_at_level(50), 142500);
        assert_eq!(LevelingRate::Fluctuating.exp_at_level(77), 639146);
        assert_eq!(LevelingRate::Fluctuating.exp_at_level(100), 1640000);
    }

    #[test]
    fn calculates_level_from_exp() {
        assert_eq!(LevelingRate::Erratic.level_from_exp(0), 1);
        assert_eq!(LevelingRate::Erratic.level_from_exp(14), 1);
        assert_eq!(LevelingRate::Erratic.level_from_exp(15), 2);
        assert_eq!(LevelingRate::Erratic.level_from_exp(209728), 62);
        assert_eq!(LevelingRate::Erratic.level_from_exp(217539), 62);
        assert_eq!(LevelingRate::Erratic.level_from_exp(217540), 63);
        assert_eq!(LevelingRate::Erratic.level_from_exp(600000), 100);

        assert_eq!(LevelingRate::Slow.level_from_exp(0), 1);
        assert_eq!(LevelingRate::Slow.level_from_exp(9), 1);
        assert_eq!(LevelingRate::Slow.level_from_exp(10), 2);
        assert_eq!(LevelingRate::Slow.level_from_exp(297910), 62);
        assert_eq!(LevelingRate::Slow.level_from_exp(312557), 62);
        assert_eq!(LevelingRate::Slow.level_from_exp(312558), 63);
        assert_eq!(LevelingRate::Slow.level_from_exp(1250000), 100);
    }
}
