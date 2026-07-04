use anyhow::Result;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    BattlePhase,
    Field,
    Log,
    alter_battle_state_from_log,
    ui,
};

/// The state of a battle.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleState {
    pub phase: BattlePhase,
    pub turn: usize,
    pub winning_side: Option<usize>,
    pub last_log_index: usize,
    pub battle_type: String,
    pub field: Field,
    pub ui_log: Vec<Vec<ui::UiLogEntry>>,
}

/// Alters the battle state according to the battle log.
pub fn alter_battle_state(state: BattleState, log: &Log) -> Result<BattleState> {
    let mut state = state;
    alter_battle_state_from_log(&mut state, log, log.current_turn())?;
    Ok(state)
}

/// Alters the battle state according to the battle log, up to the given turn.
pub fn alter_battle_state_up_to_turn(
    state: BattleState,
    log: &Log,
    turn: usize,
) -> Result<BattleState> {
    let mut state = state;
    alter_battle_state_from_log(&mut state, log, turn)?;
    Ok(state)
}
