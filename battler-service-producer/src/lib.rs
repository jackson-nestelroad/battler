#[macro_export]
macro_rules! log_procedure {
    ($name:expr, $details:expr, $block:expr) => {{
        let name = $name;
        let details = $details;
        log::info!("RPC: {} invoked ({})", name, details);
        let start = std::time::Instant::now();
        let res = $block;
        let elapsed = start.elapsed();
        match &res {
            Ok(_) => log::info!("RPC: {} succeeded ({}) [{:?}]", name, details, elapsed),
            Err(err) => log::error!(
                "RPC: {} failed ({}) [{:?}]: {:?}",
                name,
                details,
                elapsed,
                err
            ),
        }
        res
    }};
}

mod common;
mod handlers;
mod producer;

pub use common::auth::{
    BattleAuthorizer,
    BattleOperation,
    PlayerOperation,
    authorize_battle_owner,
    authorize_player,
    authorize_player_or_side,
    authorize_side,
};
pub use producer::{
    Modules,
    run_battler_service_producer,
    run_battler_service_producer_over_service,
};
