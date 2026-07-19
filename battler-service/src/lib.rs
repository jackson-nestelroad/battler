mod api;
mod error;
mod log;
mod service;
mod timer;

pub use api::*;
pub use error::BattleError;
pub use log::{
    GlobalLogEntry,
    Log,
    LogEntry,
};
pub use service::{
    BattleServiceOptions,
    BattlerService,
};
pub use timer::{
    Timer,
    Timers,
};

#[cfg(all(test, feature = "typescript"))]
mod typescript_tests {
    use ts_rs::TS;

    use super::*;

    #[test]
    fn export_types() {
        PlayerState::export().unwrap();
        PlayerValidation::export().unwrap();
        Player::export().unwrap();
        Side::export().unwrap();
        BattleState::export().unwrap();
        BattleStatus::export().unwrap();
        BattleMetadata::export().unwrap();
        Battle::export().unwrap();
        PlayerPreview::export().unwrap();
        SidePreview::export().unwrap();
        BattlePreview::export().unwrap();
        LogEntry::export().unwrap();
        Timer::export().unwrap();
        Timers::export().unwrap();
        BattleServiceOptions::export().unwrap();
    }
}
