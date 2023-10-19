use serde::{
    Deserialize,
    Serialize,
};

use crate::mons::{
    Gender,
    Nature,
    StatTable,
    Type,
};

/// Data about a specific Mon on a team.
///
/// Data here is consistent across many battles and should not be modified inside of a battle. For
/// example, if a Mon changes its ability in a battle, the `ability` field here should not be
/// updated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonData {
    /// Nickname.
    ///
    /// If not given, the name should be equal to the species name.
    pub name: String,
    /// Species name, including forme if applicable.
    pub species: String,
    /// Held item.
    pub item: Option<String>,
    /// Ability.
    pub ability: String,
    /// Moves.
    pub moves: Vec<String>,
    #[serde(default)]
    // PP boosts.
    pub pp_boosts: Vec<u8>,
    /// Nature.
    pub nature: Nature,
    /// Gender.
    pub gender: Gender,
    /// Effort values, which boost stats.
    #[serde(default)]
    pub evs: StatTable,
    /// Individual values, which boost stats.
    #[serde(default)]
    pub ivs: StatTable,
    /// Level, typically between 1 and 100.
    #[serde(default)]
    pub level: u8,
    /// Shiny?
    #[serde(default)]
    pub shiny: bool,
    /// Happiness value.
    #[serde(default)]
    pub happiness: u8,
    /// Type of ball the Mon is stored in.
    pub ball: String,
    /// Hidden power type.
    pub hidden_power_type: Option<Type>,
}
