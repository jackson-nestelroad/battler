use anyhow::Error;
use battler_service_schema::BattlerServiceError;
use battler_wamp::core::error::WampError;
use battler_wamp_uri::Uri;
use battler_wamp_values::{
    Dictionary,
    List,
    Value,
};

/// Maps [`BattleError::NotFound`] to the [`BattlerServiceError::BattleNotFound`] WAMP error.
pub fn map_battle_error(err: Error) -> Error {
    if let Some(battler_service::BattleError::NotFound) =
        err.downcast_ref::<battler_service::BattleError>()
    {
        Error::from(Into::<WampError>::into(BattlerServiceError::BattleNotFound))
    } else if let Some(validation_error) = err.downcast_ref::<battler::error::ValidationError>() {
        let mut arguments = List::new();
        for problem in validation_error.problems() {
            arguments.push(Value::String(problem.to_string()));
        }
        Error::from(WampError::new_with_payload(
            Uri::try_from("com.battler.battler_service.error.validation_failed").unwrap(),
            format!("{validation_error}"),
            arguments,
            Dictionary::default(),
        ))
    } else {
        err
    }
}
