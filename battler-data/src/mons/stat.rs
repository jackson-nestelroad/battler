use core::fmt::Debug;

use hashbrown::HashMap;
use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

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
}

/// A map of values for each stat.
pub type StatMap<T> = HashMap<Stat, T>;

/// A table of stat values.
pub type PartialStatTable = StatMap<u16>;

/// Trait for getting a stat from a container.
pub trait ContainsOptionalStats<T> {
    fn get_stat(&self, stat: Stat) -> Option<(Stat, T)>;
}

/// Iterator type for iterating over [`Stat`]s in a consistent order.
pub struct StatOrderIterator {
    next: Option<Stat>,
}

impl StatOrderIterator {
    /// Creates a new stat iterator.
    pub fn new() -> Self {
        Self {
            next: Some(Stat::HP),
        }
    }

    fn next_internal(&mut self) -> Option<Stat> {
        let out = self.next;
        self.next = match self.next {
            Some(Stat::HP) => Some(Stat::Atk),
            Some(Stat::Atk) => Some(Stat::Def),
            Some(Stat::Def) => Some(Stat::SpAtk),
            Some(Stat::SpAtk) => Some(Stat::SpDef),
            Some(Stat::SpDef) => Some(Stat::Spe),
            None | Some(Stat::Spe) => None,
        };
        out
    }
}

impl Iterator for StatOrderIterator {
    type Item = Stat;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_internal()
    }
}

/// Iterator over the entries of a [`StatTable`] (or similar container) in a stable order.
pub struct StatTableEntries<'s, S, T>
where
    S: ContainsOptionalStats<T>,
    T: Copy,
{
    table: &'s S,
    stat_iter: StatOrderIterator,
    _phantom: core::marker::PhantomData<T>,
}

impl<'s, S, T> StatTableEntries<'s, S, T>
where
    S: ContainsOptionalStats<T>,
    T: Copy,
{
    /// Creates a new iterator over the entries of a [`StatTable`].
    pub fn new(table: &'s S) -> Self {
        Self {
            table,
            stat_iter: StatOrderIterator::new(),
            _phantom: core::marker::PhantomData,
        }
    }

    fn next_entry(&mut self) -> Option<(Stat, T)> {
        while let Some(stat) = self.stat_iter.next() {
            let entry = self.table.get_stat(stat);
            if entry.is_some() {
                return entry;
            }
        }
        None
    }
}

impl<'s, S, T> Iterator for StatTableEntries<'s, S, T>
where
    S: ContainsOptionalStats<T>,
    T: Copy,
{
    type Item = (Stat, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.next_entry()
    }
}

/// A full stat table.
///
/// Similar to [`PartialStatTable`], but all values must be defined.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    /// Sets the given value in the stat table.
    pub fn set(&mut self, stat: Stat, value: u16) {
        let stat = match stat {
            Stat::HP => &mut self.hp,
            Stat::Atk => &mut self.atk,
            Stat::Def => &mut self.def,
            Stat::SpAtk => &mut self.spa,
            Stat::SpDef => &mut self.spd,
            Stat::Spe => &mut self.spe,
        };
        *stat = value;
    }

    /// Creates an iterator over all stat entries.
    pub fn iter<'s>(&'s self) -> StatTableEntries<'s, Self, u16> {
        StatTableEntries::new(self)
    }

    /// Alias for [`Self::iter`].
    pub fn entries<'s>(&'s self) -> StatTableEntries<'s, Self, u16> {
        self.iter()
    }

    /// Creates an iterator over all stat values.
    pub fn values<'s>(&'s self) -> impl Iterator<Item = u16> + 's {
        self.iter().map(|(_, value)| value)
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

    /// Copies the stat tale without the HP stat.
    pub fn without_hp(&self) -> PartialStatTable {
        PartialStatTable::from_iter([
            (Stat::Atk, self.atk),
            (Stat::Def, self.def),
            (Stat::SpAtk, self.spa),
            (Stat::SpDef, self.spd),
            (Stat::Spe, self.spe),
        ])
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

impl FromIterator<(Stat, u16)> for StatTable {
    fn from_iter<T: IntoIterator<Item = (Stat, u16)>>(iter: T) -> Self {
        let mut out = StatTable::default();
        for (stat, value) in iter {
            out.set(stat, value);
        }
        out
    }
}

impl<T: Copy> ContainsOptionalStats<T> for StatMap<T> {
    fn get_stat(&self, stat: Stat) -> Option<(Stat, T)> {
        self.get(&stat).map(|val| (stat, *val))
    }
}

impl ContainsOptionalStats<u16> for StatTable {
    fn get_stat(&self, stat: Stat) -> Option<(Stat, u16)> {
        Some((stat, self.get(stat)))
    }
}

impl<'s> IntoIterator for &'s StatTable {
    type IntoIter = StatTableEntries<'s, StatTable, u16>;
    type Item = (Stat, u16);
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod stat_test {
    use crate::{
        StatOrderIterator,
        mons::Stat,
        test_util::{
            test_string_deserialization,
            test_string_serialization,
        },
    };

    #[test]
    fn iterates_in_order() {
        let mut iter = StatOrderIterator::new();
        assert_eq!(iter.next(), Some(Stat::HP));
        assert_eq!(iter.next(), Some(Stat::Atk));
        assert_eq!(iter.next(), Some(Stat::Def));
        assert_eq!(iter.next(), Some(Stat::SpAtk));
        assert_eq!(iter.next(), Some(Stat::SpDef));
        assert_eq!(iter.next(), Some(Stat::Spe));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn serializes_to_string() {
        test_string_serialization(Stat::HP, "hp");
        test_string_serialization(Stat::Atk, "atk");
        test_string_serialization(Stat::Def, "def");
        test_string_serialization(Stat::SpAtk, "spa");
        test_string_serialization(Stat::SpDef, "spd");
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
mod stat_table_test {
    use crate::{
        PartialStatTable,
        Stat,
        StatTable,
    };

    #[test]
    fn converts_from_partial_stat_table() {
        let mut table = PartialStatTable::default();
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
    fn sets_associated_value() {
        let mut st = StatTable {
            hp: 1,
            atk: 2,
            def: 3,
            spa: 4,
            spd: 5,
            spe: 6,
        };
        st.set(Stat::HP, 2);
        st.set(Stat::Atk, 4);
        st.set(Stat::Def, 6);
        st.set(Stat::SpAtk, 8);
        st.set(Stat::SpDef, 10);
        st.set(Stat::Spe, 12);
        assert_eq!(st.get(Stat::HP), 2);
        assert_eq!(st.get(Stat::Atk), 4);
        assert_eq!(st.get(Stat::Def), 6);
        assert_eq!(st.get(Stat::SpAtk), 8);
        assert_eq!(st.get(Stat::SpDef), 10);
        assert_eq!(st.get(Stat::Spe), 12);
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

    #[test]
    fn from_iter_constructs_table() {
        let st = StatTable::from_iter([
            (Stat::HP, 108),
            (Stat::Atk, 130),
            (Stat::Def, 95),
            (Stat::SpAtk, 80),
            (Stat::SpDef, 85),
            (Stat::Spe, 102),
        ]);
        assert_eq!(
            st,
            StatTable {
                hp: 108,
                atk: 130,
                def: 95,
                spa: 80,
                spd: 85,
                spe: 102,
            }
        )
    }
}
