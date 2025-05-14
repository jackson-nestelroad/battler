use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::Gender;

/// Common evolution methods.
///
/// This enum is encoded as a single character: `L`, `T`, `I`, `B`, or `C`.
#[derive(Debug, Clone, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum EvolutionMethod {
    /// Evolves on level-up.
    #[string = "L"]
    #[alias = "Level"]
    Level,
    /// Evoles after being traded.
    #[string = "T"]
    #[alias = "Trade"]
    Trade,
    /// Evolves on item use outside of battle.
    #[string = "I"]
    #[alias = "Item"]
    Item,
    /// Evolves after a battle ends, regardless of if a level up occurred.
    #[string = "B"]
    #[alias = "Battle"]
    BattleEnd,
    /// Evolves in some other custom way.
    #[string = "C"]
    #[alias = "Custom"]
    Custom,
}

/// Details and conditions for one species to evolve into another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionData {
    /// The evolution method, which determines when the rest of the conditions should be checked.
    pub method: EvolutionMethod,
    /// A string detailing how the species evolves.
    pub condition: String,
    /// Minimum level that must be reached for evolution.
    pub level: Option<u8>,
    /// Does the species require high friendship?
    ///
    /// High friendship is defined as a value of at least 220.
    pub friendship: Option<bool>,
    /// Move that must be present on the Mon's moveset.
    pub knows_move: Option<String>,
    /// Ttime of day where evolution occurs
    pub time_of_day: Option<String>,
    /// Item held by the Mon.
    pub holding_item: Option<String>,
    /// Gender of the Mon.
    pub gender: Option<Gender>,
    /// Item that must be used on the Mon.
    pub use_item: Option<String>,
    /// The species that this Mon was traded for.
    pub trade_for: Option<String>,
}

#[cfg(test)]
mod evolution_method_test {
    use crate::{
        mons::EvolutionMethod,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(EvolutionMethod::Level, "L");
        test_string_serialization(EvolutionMethod::Trade, "T");
        test_string_serialization(EvolutionMethod::Item, "I");
        test_string_serialization(EvolutionMethod::BattleEnd, "B");
        test_string_serialization(EvolutionMethod::Custom, "C");
    }

    #[test]
    fn deserializes_full_name() {
        test_string_deserialization("Level", EvolutionMethod::Level);
        test_string_deserialization("Trade", EvolutionMethod::Trade);
        test_string_deserialization("Item", EvolutionMethod::Item);
        test_string_deserialization("Battle", EvolutionMethod::BattleEnd);
        test_string_deserialization("Custom", EvolutionMethod::Custom);
    }
}
