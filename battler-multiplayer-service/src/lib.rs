mod ai;
mod api;
mod client;
mod service;

pub use api::*;
pub use client::{
    BattlerMultiplayerServiceClient,
    DirectBattlerMultiplayerServiceClient,
};
pub use service::BattlerMultiplayerService;

#[cfg(all(test, feature = "typescript"))]
mod typescript_tests {
    use ts_rs::TS;

    use super::*;

    #[test]
    fn export_types() {
        RandomOptions::export().unwrap();
        GeminiOptions::export().unwrap();
        AiPlayerType::export().unwrap();
        AiPlayerOptions::export().unwrap();
        AiPlayers::export().unwrap();
        ProposedBattleOptions::export().unwrap();
        PlayerStatus::export().unwrap();
        Player::export().unwrap();
        Side::export().unwrap();
        ProposedBattle::export().unwrap();
        ProposedBattleResponse::export().unwrap();
        ProposedBattleRejection::export().unwrap();
        ProposedBattleUpdate::export().unwrap();
    }
}
