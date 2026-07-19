use thiserror::Error;

/// An error that occurs during multiplayer service operations.
#[derive(Debug, Error)]
pub enum MultiplayerError {
    #[error("proposed battle not found")]
    ProposedBattleNotFound,
}
