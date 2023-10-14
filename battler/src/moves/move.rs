use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    common::{
        FastHashSet,
        Id,
        Identifiable,
    },
    mons::Type,
    moves::{
        Accuracy,
        MoveCategory,
        MoveFlags,
        MoveTarget,
    },
};

/// Data about a particular move.
///
/// Moves are the primary effect that drive battle forward. Every Mon enters a battle with their
/// moveset. Each turn, a Mon uses one move to affect the battle. Moves can damage opposing Mons,
/// affect ally Mons or the user itself, boost or drop stats, apply conditions to Mons or the
/// battlefield itself, and more.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveData {
    /// Name of the move.
    pub name: String,
    /// Move category.
    pub category: MoveCategory,
    /// Move type.
    pub primary_type: Type,
    /// Base power.
    #[serde(default)]
    pub base_power: u32,
    /// Base accuracy.
    pub accuracy: Accuracy,
    /// Total power points, which is the number of times this move can be used.
    pub pp: u8,
    /// Move priority.
    #[serde(default)]
    pub priority: i8,
    /// Move target(s).
    pub target: MoveTarget,
    /// Move flags.
    pub flags: FastHashSet<MoveFlags>,
}

/// An inidividual move, which can be used by a Mon in battle.
#[derive(Clone)]
pub struct Move {
    /// Move data.
    pub data: MoveData,
    id: Id,
}

impl Move {
    /// Creates a new [`Move`] instance from [`MoveData`].
    pub fn new(data: MoveData) -> Self {
        let id = Id::from(data.name.as_ref());
        Self { data, id }
    }
}

impl Identifiable for Move {
    fn id(&self) -> &Id {
        &self.id
    }
}
