use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::{
    battler_error,
    common::{
        Error,
        FastHashMap,
    },
    mons::Stat,
};

/// A single stat value that can be boosted.
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
pub enum Boost {
    #[string = "atk"]
    #[alias = "Attack"]
    Atk,
    #[string = "def"]
    #[alias = "Defense"]
    Def,
    #[string = "spa"]
    #[alias = "spatk"]
    #[alias = "Sp.Atk"]
    #[alias = "Special Attack"]
    SpAtk,
    #[string = "spd"]
    #[alias = "spdef"]
    #[alias = "Sp.Def"]
    #[alias = "Special Defense"]
    SpDef,
    #[string = "spe"]
    #[alias = "Speed"]
    Spe,
    #[string = "acc"]
    #[alias = "Accuracy"]
    Accuracy,
    #[string = "eva"]
    #[alias = "Evasion"]
    Evasion,
}

impl TryFrom<Stat> for Boost {
    type Error = Error;
    fn try_from(value: Stat) -> Result<Self, Self::Error> {
        match value {
            Stat::HP => Err(battler_error!("HP cannot be boosted")),
            Stat::Atk => Ok(Self::Atk),
            Stat::Def => Ok(Self::Def),
            Stat::SpAtk => Ok(Self::SpAtk),
            Stat::SpDef => Ok(Self::SpDef),
            Stat::Spe => Ok(Self::Spe),
        }
    }
}

/// A map of values for each boostable stat.
pub type BoostMap<T> = FastHashMap<Boost, T>;

/// A table of boost values.
pub type PartialBoostTable = BoostMap<i8>;

/// A full boost table.
///
/// Similar to [`PartialBoostTable`], but all values must be defined.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoostTable {
    #[serde(default)]
    pub atk: i8,
    #[serde(default)]
    pub def: i8,
    #[serde(default)]
    pub spa: i8,
    #[serde(default)]
    pub spd: i8,
    #[serde(default)]
    pub spe: i8,
    #[serde(default)]
    pub acc: i8,
    #[serde(default)]
    pub eva: i8,
}

impl BoostTable {
    /// Returns the value for the given boost.
    pub fn get(&self, boost: Boost) -> i8 {
        match boost {
            Boost::Atk => self.atk,
            Boost::Def => self.def,
            Boost::SpAtk => self.spa,
            Boost::SpDef => self.spd,
            Boost::Spe => self.spe,
            Boost::Accuracy => self.acc,
            Boost::Evasion => self.eva,
        }
    }

    /// Returns a mutable reference for the given boost.
    fn get_mut(&mut self, boost: Boost) -> &mut i8 {
        match boost {
            Boost::Atk => &mut self.atk,
            Boost::Def => &mut self.def,
            Boost::SpAtk => &mut self.spa,
            Boost::SpDef => &mut self.spd,
            Boost::Spe => &mut self.spe,
            Boost::Accuracy => &mut self.acc,
            Boost::Evasion => &mut self.eva,
        }
    }

    /// Sets the value for the given boost.
    pub fn set(&mut self, boost: Boost, value: i8) {
        *self.get_mut(boost) = value;
    }
}

impl From<&PartialBoostTable> for BoostTable {
    fn from(value: &PartialBoostTable) -> Self {
        Self {
            atk: *value.get(&Boost::Atk).unwrap_or(&0),
            def: *value.get(&Boost::Def).unwrap_or(&0),
            spa: *value.get(&Boost::SpAtk).unwrap_or(&0),
            spd: *value.get(&Boost::SpDef).unwrap_or(&0),
            spe: *value.get(&Boost::Spe).unwrap_or(&0),
            acc: *value.get(&Boost::Accuracy).unwrap_or(&0),
            eva: *value.get(&Boost::Evasion).unwrap_or(&0),
        }
    }
}

/// Iterator type for iterating over boosts in a [`BoostMap`] in a stable order.
pub struct BoostMapInOrderIterator<'m, T> {
    table: &'m BoostMap<T>,
    next: Option<Boost>,
}

impl<'m, T> BoostMapInOrderIterator<'m, T> {
    pub fn new(table: &'m BoostMap<T>) -> Self {
        Self {
            table,
            next: Some(Boost::Atk),
        }
    }

    fn next_entry(&self, current: &Option<Boost>) -> (Option<(&'m Boost, &'m T)>, Option<Boost>) {
        let next = match current {
            Some(Boost::Atk) => Some(Boost::Def),
            Some(Boost::Def) => Some(Boost::SpAtk),
            Some(Boost::SpAtk) => Some(Boost::SpDef),
            Some(Boost::SpDef) => Some(Boost::Spe),
            Some(Boost::Spe) => Some(Boost::Accuracy),
            Some(Boost::Accuracy) => Some(Boost::Evasion),
            None | Some(Boost::Evasion) => None,
        };
        (
            current
                .map(|boost| self.table.get_key_value(&boost))
                .flatten(),
            next,
        )
    }

    fn next_non_zero_entry(&mut self) -> Option<(&'m Boost, &'m T)> {
        while self.next.is_some() {
            let (entry, next) = self.next_entry(&self.next);
            self.next = next;
            if entry.is_some() {
                return entry;
            }
        }
        None
    }
}

impl<'m, T> Iterator for BoostMapInOrderIterator<'m, T> {
    type Item = (&'m Boost, &'m T);
    fn next(&mut self) -> Option<Self::Item> {
        self.next_non_zero_entry()
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
        test_string_serialization(Boost::SpAtk, "spa");
        test_string_serialization(Boost::SpDef, "spd");
        test_string_serialization(Boost::Spe, "spe");
        test_string_serialization(Boost::Accuracy, "acc");
        test_string_serialization(Boost::Evasion, "eva");
    }

    #[test]
    fn deserializes_capitalized() {
        test_string_deserialization("Atk", Boost::Atk);
        test_string_deserialization("Def", Boost::Def);
        test_string_deserialization("SpAtk", Boost::SpAtk);
        test_string_deserialization("SpDef", Boost::SpDef);
        test_string_deserialization("Spe", Boost::Spe);
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
        test_string_deserialization("Speed", Boost::Spe);
        test_string_deserialization("Accuracy", Boost::Accuracy);
        test_string_deserialization("Evasion", Boost::Evasion);
    }
}

#[cfg(test)]
mod boost_table_tests {
    use ahash::HashMapExt;

    use crate::battle::{
        Boost,
        BoostMapInOrderIterator,
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
                spa: 0,
                spd: 0,
                spe: 0,
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
            spa: 3,
            spd: 4,
            spe: 5,
            acc: 6,
            eva: 7,
        };
        assert_eq!(bt.get(Boost::Atk), 1);
        assert_eq!(bt.get(Boost::Def), 2);
        assert_eq!(bt.get(Boost::SpAtk), 3);
        assert_eq!(bt.get(Boost::SpDef), 4);
        assert_eq!(bt.get(Boost::Spe), 5);
        assert_eq!(bt.get(Boost::Accuracy), 6);
        assert_eq!(bt.get(Boost::Evasion), 7);
    }

    #[test]
    fn iterates_entries_in_order() {
        let mut table = PartialBoostTable::new();
        assert_eq!(
            BoostMapInOrderIterator::new(&table)
                .map(|(boost, val)| (*boost, *val))
                .collect::<Vec<(Boost, i8)>>(),
            Vec::<(Boost, i8)>::new(),
        );

        table.insert(Boost::SpAtk, 1);
        assert_eq!(
            BoostMapInOrderIterator::new(&table)
                .map(|(boost, val)| (*boost, *val))
                .collect::<Vec<(Boost, i8)>>(),
            vec![(Boost::SpAtk, 1)],
        );

        table.insert(Boost::Atk, 2);
        assert_eq!(
            BoostMapInOrderIterator::new(&table)
                .map(|(boost, val)| (*boost, *val))
                .collect::<Vec<(Boost, i8)>>(),
            vec![(Boost::Atk, 2), (Boost::SpAtk, 1)],
        );

        table.insert(Boost::Accuracy, -1);
        assert_eq!(
            BoostMapInOrderIterator::new(&table)
                .map(|(boost, val)| (*boost, *val))
                .collect::<Vec<(Boost, i8)>>(),
            vec![(Boost::Atk, 2), (Boost::SpAtk, 1), (Boost::Accuracy, -1)],
        );

        let table = PartialBoostTable::from_iter([
            (Boost::Atk, 1),
            (Boost::Def, 1),
            (Boost::SpAtk, 1),
            (Boost::SpDef, 1),
            (Boost::Spe, 1),
            (Boost::Accuracy, 1),
            (Boost::Evasion, 1),
        ]);
        assert_eq!(
            BoostMapInOrderIterator::new(&table)
                .map(|(boost, val)| (*boost, *val))
                .collect::<Vec<(Boost, i8)>>(),
            vec![
                (Boost::Atk, 1),
                (Boost::Def, 1),
                (Boost::SpAtk, 1),
                (Boost::SpDef, 1),
                (Boost::Spe, 1),
                (Boost::Accuracy, 1),
                (Boost::Evasion, 1),
            ],
        );
    }
}
