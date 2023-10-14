use serde::{
    Deserialize,
    Serialize,
};

use crate::teams::MonData;

/// A single team for a battle, made up of one or more Mons.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TeamData {
    /// Members of the team.
    pub members: Vec<MonData>,
}
