use battler_data::{
    Id,
    MoveData,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::common::FastHashMap;

/// Local data to an fxlang effect or condition.
///
/// Data here can be referenced by callbacks in the owning effect or condition.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LocalData {
    /// Custom moves that can be used by the effect.
    #[serde(default)]
    pub moves: FastHashMap<Id, MoveData>,
}
