use std::fmt::Debug;

use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::common::FastHashMap;

/// A single stat value.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum Stat {
    #[string = "hp"]
    HP,
    #[string = "atk"]
    #[alias = "Attack"]
    Atk,
    #[string = "def"]
    #[alias = "Defense"]
    Def,
    #[string = "spatk"]
    #[alias = "spa"]
    #[alias = "Sp.Atk"]
    #[alias = "Special Attack"]
    SpAtk,
    #[string = "spdef"]
    #[alias = "spd"]
    #[alias = "Sp.Def"]
    #[alias = "Special Defense"]
    SpDef,
    #[string = "spe"]
    #[alias = "Speed"]
    Spe,
}

/// A map of values for each stat.
pub type StatMap<T> = FastHashMap<Stat, T>;

/// A table of stat values.
pub type PartialStatTable = StatMap<u8>;

/// Iterator over the value sof a [`StatTable`].
pub struct StatTableValues<'s> {
    table: &'s StatTable,
    next_stat: Option<Stat>,
}

impl<'s> StatTableValues<'s> {
    /// Creates a new iterator over the values in a [`StatTable`].
    fn new(table: &'s StatTable) -> Self {
        Self {
            table,
            next_stat: Some(Stat::HP),
        }
    }
}

impl<'s> Iterator for StatTableValues<'s> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        let next_stat = self.next_stat?;
        let value = self.table.get(next_stat);
        self.next_stat = match next_stat {
            Stat::HP => Some(Stat::Atk),
            Stat::Atk => Some(Stat::Def),
            Stat::Def => Some(Stat::SpAtk),
            Stat::SpAtk => Some(Stat::SpDef),
            Stat::SpDef => Some(Stat::Spe),
            Stat::Spe => None,
        };
        Some(value)
    }
}

/// A full stat table.
///
/// Similar to [`PartialStatTable`], but all values must be defined.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatTable {
    #[serde(default)]
    pub hp: u16,
    #[serde(default)]
    pub atk: u16,
    #[serde(default)]
    pub def: u16,
    #[serde(default)]
    pub spa: u16,
    #[serde(default)]
    pub spd: u16,
    #[serde(default)]
    pub spe: u16,
}

impl StatTable {
    /// Returns the value for the given stat.
    pub fn get(&self, stat: Stat) -> u16 {
        match stat {
            Stat::HP => self.hp,
            Stat::Atk => self.atk,
            Stat::Def => self.def,
            Stat::SpAtk => self.spa,
            Stat::SpDef => self.spd,
            Stat::Spe => self.spe,
        }
    }

    /// Creates an iterator over all stat values.
    pub fn values<'s>(&'s self) -> StatTableValues<'s> {
        StatTableValues::new(self)
    }

    /// Sums up all stats in the table.
    pub fn sum(&self) -> u32 {
        self.hp as u32
            + self.atk as u32
            + self.def as u32
            + self.spa as u32
            + self.spd as u32
            + self.spe as u32
    }
}

impl From<&PartialStatTable> for StatTable {
    fn from(value: &PartialStatTable) -> Self {
        Self {
            hp: *value.get(&Stat::HP).unwrap_or(&0) as u16,
            atk: *value.get(&Stat::Atk).unwrap_or(&0) as u16,
            def: *value.get(&Stat::Def).unwrap_or(&0) as u16,
            spa: *value.get(&Stat::SpAtk).unwrap_or(&0) as u16,
            spd: *value.get(&Stat::SpDef).unwrap_or(&0) as u16,
            spe: *value.get(&Stat::Spe).unwrap_or(&0) as u16,
        }
    }
}

#[cfg(test)]
mod stat_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        mons::Stat,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(Stat::HP, "hp");
        test_string_serialization(Stat::Atk, "atk");
        test_string_serialization(Stat::Def, "def");
        test_string_serialization(Stat::SpAtk, "spatk");
        test_string_serialization(Stat::SpDef, "spdef");
        test_string_serialization(Stat::Spe, "spe");
    }

    #[test]
    fn deserializes_capitalized() {
        test_string_deserialization("HP", Stat::HP);
        test_string_deserialization("Atk", Stat::Atk);
        test_string_deserialization("Def", Stat::Def);
        test_string_deserialization("SpAtk", Stat::SpAtk);
        test_string_deserialization("SpDef", Stat::SpDef);
        test_string_deserialization("Spe", Stat::Spe);
    }

    #[test]
    fn deserializes_full_names() {
        test_string_deserialization("Attack", Stat::Atk);
        test_string_deserialization("Defense", Stat::Def);
        test_string_deserialization("Special Attack", Stat::SpAtk);
        test_string_deserialization("Sp.Atk", Stat::SpAtk);
        test_string_deserialization("Special Defense", Stat::SpDef);
        test_string_deserialization("Sp.Def", Stat::SpDef);
        test_string_deserialization("Speed", Stat::Spe);
    }
}

#[cfg(test)]
mod stat_table_tests {
    use ahash::HashMapExt;

    use crate::mons::{
        PartialStatTable,
        Stat,
        StatTable,
    };

    #[test]
    fn converts_from_partial_stat_table() {
        let mut table = PartialStatTable::new();
        table.insert(Stat::HP, 2);
        table.insert(Stat::SpDef, 255);
        let table = StatTable::from(&table);
        assert_eq!(
            table,
            StatTable {
                hp: 2,
                atk: 0,
                def: 0,
                spa: 0,
                spd: 255,
                spe: 0,
            }
        )
    }

    #[test]
    fn gets_associated_value() {
        let st = StatTable {
            hp: 1,
            atk: 2,
            def: 3,
            spa: 4,
            spd: 5,
            spe: 6,
        };
        assert_eq!(st.get(Stat::HP), 1);
        assert_eq!(st.get(Stat::Atk), 2);
        assert_eq!(st.get(Stat::Def), 3);
        assert_eq!(st.get(Stat::SpAtk), 4);
        assert_eq!(st.get(Stat::SpDef), 5);
        assert_eq!(st.get(Stat::Spe), 6);
    }

    #[test]
    fn sums() {
        let st = StatTable {
            hp: 100,
            atk: 120,
            def: 120,
            spa: 150,
            spd: 100,
            spe: 90,
        };
        assert_eq!(st.sum(), 680);
    }

    #[test]
    fn values_iterates_over_all_values() {
        let st = StatTable {
            hp: 100,
            atk: 120,
            def: 120,
            spa: 150,
            spd: 100,
            spe: 90,
        };
        assert!(st.values().all(|val| val < 255));
        assert_eq!(st.values().sum::<u16>(), 680);
    }
}
