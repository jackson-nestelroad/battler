use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    common::{
        FastHashMap,
        Id,
    },
    moves::MoveData,
};

/// Local data to an fxlang effect or condition.
///
/// Data here can be referenced by callbacks in the owning effect or condition.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LocalData {
    #[serde(default)]
    pub moves: FastHashMap<Id, MoveData>,
}
