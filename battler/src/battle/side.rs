use num::Integer;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        BattleRegistry,
        BattleType,
        MonHandle,
        Player,
        PlayerContext,
        PlayerData,
        SideContext,
    },
    common::{
        Error,
        WrapResultError,
    },
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
    pub name: String,
    pub index: usize,
}

// Construction and initialization logic.
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
            .enumerate()
            .map(|(i, data)| Player::new(data, index, i, battle_type, dex, registry))
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

// Basic getters.
impl Side {
    pub fn players<'c>(context: &'c SideContext) -> impl Iterator<Item = &'c Player> {
        context.battle().players_on_side(context.side().index)
    }

    pub fn player_in_position<'c>(context: &'c SideContext, position: usize) -> Option<&'c Player> {
        Self::players(context).find(|player| player.position == position)
    }

    pub fn player_context<'s, 'c, 'b, 'd>(
        context: &'s mut SideContext<'c, 'b, 'd>,
        position: usize,
    ) -> Result<PlayerContext<'s, 's, 'b, 'd>, Error> {
        let player = Self::player_in_position(context, position)
            .wrap_error_with_format(format_args!("side has no player in position {position}"))?
            .index;
        context.as_battle_context_mut().player_context(player)
    }

    pub fn mon_in_position(
        context: &mut SideContext,
        position: usize,
    ) -> Result<Option<MonHandle>, Error> {
        let active_per_player = context.battle().format.battle_type.active_per_player();
        let (player_position, position) = position.div_mod_floor(&active_per_player);
        let player_context = Self::player_context(context, player_position)
            .wrap_error_with_format(format_args!("position {position} is out of bounds"))?;
        Player::active_mon_handle(&player_context, position)
    }
}
