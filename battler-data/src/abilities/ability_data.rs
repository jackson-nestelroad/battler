use ahash::HashSet;
use serde::{
    Deserialize,
    Serialize,
};

use crate::AbilityFlag;

/// Data about a particular ability.
///
/// Every Mon has one ability, which affects the battle in a wide variety of ways.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityData {
    /// Name of the ability.
    pub name: String,
    /// Ability flags.
    pub flags: HashSet<AbilityFlag>,

    /// Dynamic battle effects.
    #[serde(default)]
    pub effect: serde_json::Value,
    /// Dynamic battle effects of the condition created by this ability.
    #[serde(default)]
    pub condition: serde_json::Value,
}
