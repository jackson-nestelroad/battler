mod common;
mod handlers;
mod producer;

pub use common::auth::MultiplayerBattleAuthorizer;
pub use producer::{
    Modules,
    run_multiplayer_battler_service_producer,
    run_multiplayer_battler_service_producer_over_service,
};
