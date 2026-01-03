use alloc::{
    string::String,
    vec::Vec,
};

use anyhow::Result;
use battler_data::Id;
use hashbrown::HashMap;
use num::Integer;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        BattleRegistry,
        MonHandle,
        Player,
        PlayerData,
        SideContext,
    },
    config::Format,
    dex::Dex,
    effect::fxlang,
};

/// Data about a single side of a battle.
///
/// A battle always takes place between two sides. However, there can be multiple players playing on
/// each side. Players on the same side are considered allies, while players on opposite sides are
/// considered foes.
///
/// Effects can be applied to an entire side of the battle, which impacts all Mons on all players on
/// that side of the battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

    pub conditions: HashMap<Id, fxlang::EffectState>,
    pub slot_conditions: HashMap<usize, HashMap<Id, fxlang::EffectState>>,
}

// Construction and initialization logic.
impl Side {
    /// Creates a new side.
    pub fn new(
        data: SideData,
        index: usize,
        format: &Format,
        dex: &Dex,
        registry: &BattleRegistry,
    ) -> Result<(Self, Vec<Player>)> {
        let players = data
            .players
            .into_iter()
            .enumerate()
            .map(|(i, data)| Player::new(data, index, i, format, dex, registry))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((
            Self {
                name: data.name,
                index,
                conditions: HashMap::default(),
                slot_conditions: HashMap::default(),
            },
            players,
        ))
    }
}

// Basic getters.
impl Side {
    /// Converts a player position to the player index.
    pub fn player_position_to_index(context: &SideContext, position: usize) -> Option<usize> {
        context
            .battle()
            .players_on_side(context.side().index)
            .find(|player| player.position == position)
            .map(|player| player.index)
    }

    /// Looks up the Mon in the given position on the side.
    pub fn mon_in_position(
        context: &mut SideContext,
        position: usize,
    ) -> Result<Option<MonHandle>> {
        let active_per_player = context.battle().format.battle_type.active_per_player();
        let (player_position, position) = position.div_mod_floor(&active_per_player);
        let player_context = match context.player_context(player_position) {
            Err(_) => return Ok(None),
            Ok(player_context) => player_context,
        };
        Ok(player_context.player().active_mon_handle(position))
    }

    /// Counts the number of Mons left on the side.
    pub fn mons_left(context: &mut SideContext) -> Result<usize> {
        let mut count = 0;
        for player in context
            .battle()
            .player_indices_on_side(context.side().index)
            .collect::<Vec<_>>()
        {
            count += Player::mons_left(&context.as_battle_context_mut().player_context(player)?)?;
        }
        Ok(count)
    }

    /// Counts the total health percentage left on the side.
    pub fn health_percentage_left(context: &mut SideContext) -> Result<u64> {
        let mut count = 0;
        for player in context
            .battle()
            .player_indices_on_side(context.side().index)
            .collect::<Vec<_>>()
        {
            count += Player::health_percentage_left(
                &mut context.as_battle_context_mut().player_context(player)?,
            )?;
        }
        Ok(count)
    }

    /// Counts the number of active Mons on the side.
    pub fn active_mons_count(context: &SideContext) -> usize {
        context
            .battle()
            .players_on_side(context.side().index)
            .map(|player| player.active_mon_handles().count())
            .sum()
    }

    /// Checks if the side has the given condition.
    pub fn has_condition(context: &SideContext, condition: &Id) -> bool {
        context.side().conditions.contains_key(condition)
    }
}
