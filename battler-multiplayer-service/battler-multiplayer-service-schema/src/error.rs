use battler_wamprat_error::WampError;
use thiserror::Error;

/// Errors returned by the multiplayer service producer.
#[derive(Debug, Error, WampError)]
pub enum BattlerMultiplayerServiceError {
    #[error("proposed battle not found")]
    #[uri("com.battler.battler_multiplayer_service.error.proposed_battle_not_found")]
    ProposedBattleNotFound,
}
