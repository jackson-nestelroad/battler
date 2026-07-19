use anyhow::Error;
use battler_service::BattleError;
use battler_service_schema::BattlerServiceError;
use battler_wamp::core::error::WampError;

/// Maps [`BattleError::NotFound`] to the [`BattlerServiceError::BattleNotFound`] WAMP error.
pub fn map_battle_error(err: Error) -> Error {
    if let Some(BattleError::NotFound) = err.downcast_ref::<BattleError>() {
        Error::from(Into::<WampError>::into(BattlerServiceError::BattleNotFound))
    } else {
        err
    }
}
