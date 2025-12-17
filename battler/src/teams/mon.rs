use battler_data::{
    Gender,
    Nature,
    StatTable,
    Type,
};
use serde::{
    Deserialize,
    Serialize,
};

/// Pre-battle data for a specific Mon on a team.
///
/// Data here is meant to carry from battle to battle and is cleared when a Mon is healed. In
/// competitive battles, this data would be completely unused.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonPersistentBattleData {
    pub hp: Option<u16>,
    pub move_pp: Vec<u8>,
    pub status: Option<String>,
}

/// Data about a specific Mon on a team.
///
/// Data here is consistent across many battles and should not be modified inside of a battle. For
/// example, if a Mon changes its ability in a battle, the `ability` field here should not be
/// updated.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonData {
    /// Nickname.
    ///
    /// If not given, the name should be equal to the species name.
    pub name: String,
    /// Species name, including forme if applicable.
    pub species: String,
    /// Ability.
    pub ability: String,
    /// Moves.
    pub moves: Vec<String>,
    /// Held item.
    pub item: Option<String>,
    #[serde(default)]
    // PP boosts.
    pub pp_boosts: Vec<u8>,
    /// Nature.
    #[serde(default)]
    pub nature: Nature,
    /// The true nature.
    ///
    /// A Mon's nature can be changed for battle, but its true nature is still used for some
    /// inherent properties, like flavor preferences.
    #[serde(default)]
    pub true_nature: Option<Nature>,
    /// Gender.
    #[serde(default)]
    pub gender: Gender,
    /// Effort values, which boost stats.
    #[serde(default)]
    pub evs: StatTable,
    /// Individual values, which boost stats.
    #[serde(default)]
    pub ivs: StatTable,
    /// Level, typically between 1 and 100.
    ///
    /// If unset, the level will be inferred from the experience points.
    #[serde(default)]
    pub level: u8,
    /// Experience points.
    ///
    /// Only applicable for single player battle simulations, where Mons can gain experience.
    #[serde(default)]
    pub experience: u32,
    /// Shiny?
    #[serde(default)]
    pub shiny: bool,
    /// friendship value.
    #[serde(default)]
    pub friendship: u8,
    /// Type of ball the Mon is stored in.
    #[serde(default)]
    pub ball: String,
    /// Hidden power type.
    pub hidden_power_type: Option<Type>,
    /// Different original trainer.
    #[serde(default)]
    pub different_original_trainer: bool,
    /// Dynamax level.
    #[serde(default)]
    pub dynamax_level: u8,
    /// Can Gigantamax.
    #[serde(default)]
    pub gigantamax_factor: bool,
    /// Tera Type.
    pub tera_type: Option<Type>,
    /// Persistent battle data.
    #[serde(default)]
    pub persistent_battle_data: MonPersistentBattleData,
}
