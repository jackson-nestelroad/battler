use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The type of a battle.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum BattleType {
    /// One Mon from one player battles at a time.
    #[string = "Singles"]
    #[default]
    Singles,
    /// Two Mons from one player battle at a time.
    #[string = "Doubles"]
    Doubles,
    /// One Mon from each player battles at a time.
    #[string = "Multi"]
    Multi,
    /// Three Mons from one player battles at a time.
    #[string = "Triples"]
    Triples,
}

impl BattleType {
    /// The number of active Mons per player.
    pub fn active_per_player(&self) -> usize {
        match self {
            Self::Singles => 1,
            Self::Doubles => 2,
            Self::Multi => 1,
            Self::Triples => 3,
        }
    }

    /// The minimum team size for the battle format.
    pub fn min_team_size(&self) -> usize {
        self.active_per_player()
    }

    /// The default picked team size for the battle format.
    pub fn default_picked_team_size(&self) -> usize {
        match self {
            Self::Singles => 3,
            Self::Doubles => 4,
            Self::Multi => 3,
            Self::Triples => 6,
        }
    }

    /// Returns true if the battle type supports uneven sides.
    pub fn can_have_uneven_sides(&self) -> bool {
        match self {
            Self::Multi => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod battle_type_test {
    use crate::{
        battle::BattleType,
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(BattleType::Singles, "Singles");
        test_string_serialization(BattleType::Doubles, "Doubles");
        test_string_serialization(BattleType::Multi, "Multi");
        test_string_serialization(BattleType::Triples, "Triples");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("singles", BattleType::Singles);
        test_string_deserialization("doubles", BattleType::Doubles);
        test_string_deserialization("multi", BattleType::Multi);
        test_string_deserialization("triples", BattleType::Triples);
    }
}
