use battler_wamprat_error::WampError;
use thiserror::Error;

/// Errors returned by the battle service producer.
#[derive(Debug, Error, WampError)]
pub enum BattlerServiceError {
    #[error("battle does not exist")]
    #[uri("com.battler.battler_service.error.battle_not_found")]
    BattleNotFound,
}
