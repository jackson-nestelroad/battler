use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        BattleRegistry,
        BattleType,
        Player,
        PlayerData,
    },
    common::Error,
    dex::Dex,
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
    name: String,
    index: usize,
}

// Block for getters.
impl Side {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

impl Side {
    /// Creates a new [`Side`] from [`SideData`].
    pub fn new(
        data: SideData,
        index: usize,
        battle_type: &BattleType,
        dex: &Dex,
        registry: &BattleRegistry,
    ) -> Result<(Self, Vec<Player>), Error> {
        let players = data
            .players
            .into_iter()
            .map(|data| Player::new(data, index, battle_type, dex, registry))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((
            Self {
                name: data.name,
                index,
            },
            players,
        ))
    }
}
