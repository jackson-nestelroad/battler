use ahash::HashSet;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    Gender,
    Nature,
    PartialStatTable,
    ShinyChance,
};

/// Data for a particular event, which is a special giveaway of some Mon.
///
/// Event Mons can have special moves and abilities that the species would not ordinarily have. This
/// data is stored on each species to mark Mons that would ordinarily be illegal as legal and
/// legitimate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventData {
    /// Level Mon was given away at.
    pub level: Option<u8>,
    /// Can the Mon be shiny?
    #[serde(default)]
    pub shiny: ShinyChance,
    /// Gender.
    pub gender: Option<Gender>,
    /// Nature.
    pub nature: Option<Nature>,
    /// IVs.
    #[serde(default)]
    pub ivs: PartialStatTable,
    /// Does the Mon have its hidden ability?
    #[serde(default)]
    pub hidden_ability: bool,
    /// Moves the Mon could have been given away with.
    ///
    /// Moves that are ordinarily illegal should be listed here.
    #[serde(default)]
    pub moves: HashSet<String>,
    /// Type of ball the Mon was given away in.
    pub ball: Option<String>,
}
