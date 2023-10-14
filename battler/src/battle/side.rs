use serde::{
    Deserialize,
    Serialize,
};

use crate::battle::{
    BattleRegistry,
    Player,
    PlayerData,
};

/// Data about a single side of a battle.
///
/// A battle always takes place between two sides. However, there can be multiple players playing on
/// each side. Players on the same side are considered allies, while players on opposite sides are
/// considered foes.
///
/// Effects can be applied to an entire side of the battle, which impacts all Mons on all players on
/// that side of the battle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideData {
    /// Side name.
    pub name: String,
    /// Players participating on the side.
    pub players: Vec<PlayerData>,
}

/// A single side of a battle.
///
/// See [`SideData`] for details.
pub struct Side {
    pub name: String,
    pub index: usize,
}

impl Side {
    /// Creates a new [`Side`] from [`SideData`].
    pub fn new(data: SideData, index: usize, registry: &BattleRegistry) -> (Self, Vec<Player>) {
        let players = data
            .players
            .into_iter()
            .map(|data| Player::new(data, index, registry))
            .collect();
        (
            Self {
                name: data.name,
                index,
            },
            players,
        )
    }
}
