use alloc::{
    string::String,
    vec::Vec,
};

use hashbrown::HashMap;
use serde::{
    Deserialize,
    Serialize,
};

use crate::teams::MonData;

/// Data for a single player's bag in a battle.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BagData {
    /// Item counts available for use.
    pub items: HashMap<String, u16>,
}

/// A single team for a battle.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamData {
    /// Members of the team.
    pub members: Vec<MonData>,
    /// Items available for use.
    #[serde(default)]
    pub bag: BagData,
}
