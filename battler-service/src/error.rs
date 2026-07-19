use thiserror::Error;

/// An error that occurs during service-level battle operations.
#[derive(Debug, Error)]
pub enum BattleError {
    #[error("battle does not exist")]
    NotFound,
}
