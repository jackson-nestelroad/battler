use ahash::HashSet;
use battler_data::{
    BoostTable,
    Gender,
    Nature,
    StatTable,
    Type,
};

#[derive(Debug, Default, Clone)]
pub struct Field {
    pub weather: Option<String>,
    pub terrain: Option<String>,
    pub conditions: HashSet<String>,
    pub attacker_side: Side,
    pub defender_side: Side,
}

impl Field {
    pub fn has_condition<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        iter.into_iter()
            .any(|val| self.conditions.contains(val.as_ref()))
    }

    pub fn has_terrain<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if let Some(terrain) = &self.terrain {
            iter.into_iter().any(|val| val.as_ref() == terrain)
        } else {
            false
        }
    }

    pub fn has_weather<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if let Some(weather) = &self.weather {
            iter.into_iter().any(|val| val.as_ref() == weather)
        } else {
            false
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Side {
    pub conditions: HashSet<String>,
}

impl Side {
    pub fn has_condition<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        iter.into_iter()
            .any(|val| self.conditions.contains(val.as_ref()))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Mon {
    pub name: String,
    pub side: usize,
    pub level: u64,
    pub hp: Option<u64>,
    pub ability: Option<String>,
    pub item: Option<String>,
    pub gender: Option<Gender>,
    pub nature: Option<Nature>,
    pub ivs: Option<StatTable>,
    pub evs: Option<StatTable>,
    pub boosts: BoostTable,
    pub status: Option<String>,
    pub types: Vec<Type>,
    pub conditions: HashSet<String>,
}

impl Mon {
    pub fn has_ability<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if let Some(ability) = &self.ability {
            iter.into_iter().any(|val| val.as_ref() == ability)
        } else {
            false
        }
    }

    pub fn has_condition<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        iter.into_iter()
            .any(|val| self.conditions.contains(val.as_ref()))
    }

    pub fn has_item<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if let Some(item) = &self.item {
            iter.into_iter().any(|val| val.as_ref() == item)
        } else {
            false
        }
    }

    pub fn has_status<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if let Some(status) = &self.status {
            iter.into_iter().any(|val| val.as_ref() == status)
        } else {
            false
        }
    }

    pub fn has_type<I>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = Type>,
    {
        iter.into_iter().any(|val| self.types.contains(&val))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Move {
    pub name: String,
    pub spread: bool,
    pub crit: bool,
    pub hits: Option<u64>,
}

impl Move {
    pub fn is_named<I, S>(&self, iter: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        iter.into_iter().any(|val| val.as_ref() == self.name)
    }
}
