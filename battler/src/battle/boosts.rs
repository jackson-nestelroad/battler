use std::marker::PhantomData;

use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::{
    error::{
        general_error,
        Error,
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
            Stat::HP => Err(general_error("HP cannot be boosted")),
            Stat::Atk => Ok(Self::Atk),
            Stat::Def => Ok(Self::Def),
            Stat::SpAtk => Ok(Self::SpAtk),
            Stat::SpDef => Ok(Self::SpDef),
            Stat::Spe => Ok(Self::Spe),
        }
    }
}

/// Trait for getting a boosted stat from a container.
pub trait ContainsOptionalBoosts<T> {
    fn get_boost(&self, boost: Boost) -> Option<(Boost, T)>;
}

/// A full boost table.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Creates a new boost table.
    pub fn new() -> Self {
        Self::default()
    }

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

    /// Creates an iterator over all entries in the table.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (Boost, i8)> + 'a {
        BoostTableEntries::new(self)
    }

    /// Creates an iterator over all non-zero entries in the table.
    pub fn non_zero_iter<'a>(&'a self) -> impl Iterator<Item = (Boost, i8)> + 'a {
        self.iter().filter(|(_, val)| *val != 0)
    }

    /// Creates an iterator over all values of the table.
    pub fn values<'a>(&'a self) -> impl Iterator<Item = i8> + 'a {
        self.iter().map(|(_, val)| val)
    }
}

impl FromIterator<(Boost, i8)> for BoostTable {
    fn from_iter<T: IntoIterator<Item = (Boost, i8)>>(iter: T) -> Self {
        let mut table = Self::new();
        for (boost, value) in iter {
            *table.get_mut(boost) = value;
        }
        table
    }
}

impl ContainsOptionalBoosts<i8> for BoostTable {
    fn get_boost(&self, boost: Boost) -> Option<(Boost, i8)> {
        Some((boost, self.get(boost)))
    }
}

/// Iterator type for iterating over [`Boost`]s in a consistent order.
pub struct BoostOrderIterator {
    next: Option<Boost>,
}

impl BoostOrderIterator {
    /// Creates a new boost iterator.
    pub fn new() -> Self {
        Self {
            next: Some(Boost::Atk),
        }
    }

    fn next_internal(&mut self) -> Option<Boost> {
        let out = self.next;
        self.next = match self.next {
            Some(Boost::Atk) => Some(Boost::Def),
            Some(Boost::Def) => Some(Boost::SpAtk),
            Some(Boost::SpAtk) => Some(Boost::SpDef),
            Some(Boost::SpDef) => Some(Boost::Spe),
            Some(Boost::Spe) => Some(Boost::Accuracy),
            Some(Boost::Accuracy) => Some(Boost::Evasion),
            None | Some(Boost::Evasion) => None,
        };
        out
    }
}

impl Iterator for BoostOrderIterator {
    type Item = Boost;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_internal()
    }
}

/// Iterator type for iterating over non-zero boosts in a [`BoostTable`] (or similar container) in a
/// stable order.
pub struct BoostTableEntries<'m, B, T>
where
    B: ContainsOptionalBoosts<T>,
    T: Copy,
{
    table: &'m B,
    boost_iter: BoostOrderIterator,
    _phantom: PhantomData<T>,
}

impl<'m, B, T> BoostTableEntries<'m, B, T>
where
    B: ContainsOptionalBoosts<T>,
    T: Copy,
{
    /// Creates a new iterator over a boost table.
    pub fn new(table: &'m B) -> Self {
        Self {
            table,
            boost_iter: BoostOrderIterator::new(),
            _phantom: PhantomData,
        }
    }

    fn next_non_zero_entry(&mut self) -> Option<(Boost, T)> {
        while let Some(boost) = self.boost_iter.next() {
            let entry = self.table.get_boost(boost);
            if entry.is_some() {
                return entry;
            }
        }
        None
    }
}

impl<'m, B, T> Iterator for BoostTableEntries<'m, B, T>
where
    B: ContainsOptionalBoosts<T>,
    T: Copy,
{
    type Item = (Boost, T);
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

    use crate::battle::{
        Boost,
        BoostTable,
    };

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
        let mut table = BoostTable::new();
        assert_eq!(
            table.non_zero_iter().collect::<Vec<(Boost, i8)>>(),
            Vec::<(Boost, i8)>::new(),
        );

        *table.get_mut(Boost::SpAtk) = 1;
        assert_eq!(
            table.non_zero_iter().collect::<Vec<(Boost, i8)>>(),
            vec![(Boost::SpAtk, 1)],
        );

        *table.get_mut(Boost::Atk) = 2;
        assert_eq!(
            table.non_zero_iter().collect::<Vec<(Boost, i8)>>(),
            vec![(Boost::Atk, 2), (Boost::SpAtk, 1)],
        );

        *table.get_mut(Boost::Accuracy) = -1;
        assert_eq!(
            table.non_zero_iter().collect::<Vec<(Boost, i8)>>(),
            vec![(Boost::Atk, 2), (Boost::SpAtk, 1), (Boost::Accuracy, -1)],
        );

        let table = BoostTable::from_iter([
            (Boost::Atk, 1),
            (Boost::Def, 1),
            (Boost::SpAtk, 1),
            (Boost::SpDef, 1),
            (Boost::Spe, 1),
            (Boost::Accuracy, 1),
            (Boost::Evasion, 1),
        ]);
        assert_eq!(
            table.iter().collect::<Vec<(Boost, i8)>>(),
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
