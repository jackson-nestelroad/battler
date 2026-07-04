mod log_handler;
mod state;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use log_handler::alter_battle_state_from_log;
pub use state::{
    BattleState,
    alter_battle_state,
    alter_battle_state_up_to_turn,
};
pub(crate) use types::Ambiguity;
pub use types::{
    BattlePhase,
    ConditionData,
    Field,
    Mon,
    MonBattleAppearance,
    MonBattleAppearanceFromSwitchIn,
    MonBattleAppearanceReference,
    MonBattleAppearanceWithRecovery,
    MonPhysicalAppearance,
    MonVolatileData,
    Player,
    Side,
};
