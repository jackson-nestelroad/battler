use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::common::FastHashMap;

/// A single stat value that can be boosted.
#[derive(Debug, PartialEq, Eq, Hash, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum Boost {
    #[string = "atk"]
    #[alias = "Attack"]
    Atk,
    #[string = "def"]
    #[alias = "Defense"]
    Def,
    #[string = "spatk"]
    #[alias = "Sp.Atk"]
    #[alias = "Special Attack"]
    SpAtk,
    #[string = "spdef"]
    #[alias = "Sp.Def"]
    #[alias = "Special Defense"]
    SpDef,
    #[string = "spd"]
    #[alias = "Speed"]
    Spd,
    #[string = "acc"]
    #[alias = "Accuracy"]
    Accuracy,
    #[string = "eva"]
    #[alias = "Evasion"]
    Evasion,
}

/// A map of values for each boostable stat.
pub type BoostMap<T> = FastHashMap<Boost, T>;

/// A table of boost values.
pub type PartialBoostTable = BoostMap<u8>;

/// A full boost table.
///
/// Similar to [`PartialBoostTable`], but all values must be defined.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BoostTable {
    #[serde(default)]
    pub atk: u16,
    #[serde(default)]
    pub def: u16,
    #[serde(default)]
    pub spatk: u16,
    #[serde(default)]
    pub spdef: u16,
    #[serde(default)]
    pub spd: u16,
    #[serde(default)]
    pub acc: u16,
    #[serde(default)]
    pub eva: u16,
}

impl BoostTable {
    /// Returns the value for the given boost.
    pub fn get(&self, boost: Boost) -> u16 {
        match boost {
            Boost::Atk => self.atk,
            Boost::Def => self.def,
            Boost::SpAtk => self.spatk,
            Boost::SpDef => self.spdef,
            Boost::Spd => self.spd,
            Boost::Accuracy => self.acc,
            Boost::Evasion => self.eva,
        }
    }
}

impl From<&PartialBoostTable> for BoostTable {
    fn from(value: &PartialBoostTable) -> Self {
        Self {
            atk: *value.get(&Boost::Atk).unwrap_or(&0) as u16,
            def: *value.get(&Boost::Def).unwrap_or(&0) as u16,
            spatk: *value.get(&Boost::SpAtk).unwrap_or(&0) as u16,
            spdef: *value.get(&Boost::SpDef).unwrap_or(&0) as u16,
            spd: *value.get(&Boost::Spd).unwrap_or(&0) as u16,
            acc: *value.get(&Boost::Accuracy).unwrap_or(&0) as u16,
            eva: *value.get(&Boost::Evasion).unwrap_or(&0) as u16,
        }
    }
}

#[cfg(test)]
mod boost_tests {
    use crate::{
        battle::Boost,
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(Boost::Atk, "atk");
        test_string_serialization(Boost::Def, "def");
        test_string_serialization(Boost::SpAtk, "spatk");
        test_string_serialization(Boost::SpDef, "spdef");
        test_string_serialization(Boost::Spd, "spd");
        test_string_serialization(Boost::Accuracy, "acc");
        test_string_serialization(Boost::Evasion, "eva");
    }

    #[test]
    fn deserializes_capitalized() {
        test_string_deserialization("Atk", Boost::Atk);
        test_string_deserialization("Def", Boost::Def);
        test_string_deserialization("SpAtk", Boost::SpAtk);
        test_string_deserialization("SpDef", Boost::SpDef);
        test_string_deserialization("Spd", Boost::Spd);
        test_string_deserialization("Acc", Boost::Accuracy);
        test_string_deserialization("Eva", Boost::Evasion);
    }

    #[test]
    fn deserializes_full_names() {
        test_string_deserialization("Attack", Boost::Atk);
        test_string_deserialization("Defense", Boost::Def);
        test_string_deserialization("Special Attack", Boost::SpAtk);
        test_string_deserialization("Sp.Atk", Boost::SpAtk);
        test_string_deserialization("Special Defense", Boost::SpDef);
        test_string_deserialization("Sp.Def", Boost::SpDef);
        test_string_deserialization("Speed", Boost::Spd);
        test_string_deserialization("Accuracy", Boost::Accuracy);
        test_string_deserialization("Evasion", Boost::Evasion);
    }
}

#[cfg(test)]
mod boost_table_tests {
    use ahash::HashMapExt;

    use crate::battle::{
        Boost,
        BoostTable,
        PartialBoostTable,
    };

    #[test]
    fn converts_from_partial_boost_table() {
        let mut table = PartialBoostTable::new();
        table.insert(Boost::Atk, 2);
        table.insert(Boost::Accuracy, 1);
        let table = BoostTable::from(&table);
        assert_eq!(
            table,
            BoostTable {
                atk: 2,
                def: 0,
                spatk: 0,
                spdef: 0,
                spd: 0,
                acc: 1,
                eva: 0,
            }
        )
    }

    #[test]
    fn gets_associated_value() {
        let bt = BoostTable {
            atk: 1,
            def: 2,
            spatk: 3,
            spdef: 4,
            spd: 5,
            acc: 6,
            eva: 7,
        };
        assert_eq!(bt.get(Boost::Atk), 1);
        assert_eq!(bt.get(Boost::Def), 2);
        assert_eq!(bt.get(Boost::SpAtk), 3);
        assert_eq!(bt.get(Boost::SpDef), 4);
        assert_eq!(bt.get(Boost::Spd), 5);
        assert_eq!(bt.get(Boost::Accuracy), 6);
        assert_eq!(bt.get(Boost::Evasion), 7);
    }
}
