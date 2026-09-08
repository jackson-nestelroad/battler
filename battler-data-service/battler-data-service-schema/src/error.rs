use battler_wamprat_error::WampError;
use thiserror::Error;

/// Errors returned by the data service producer.
#[derive(Debug, Error, WampError)]
pub enum BattlerDataServiceError {
    #[error("resource \"{0}\" not found")]
    #[uri("com.battler.data_service.error.not_found")]
    NotFound(String),

    #[error("invalid query: {0}")]
    #[uri("com.battler.data_service.error.invalid_query")]
    InvalidQuery(String),
}
