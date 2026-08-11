#[macro_export]
macro_rules! log_procedure {
    ($name:expr, $details:expr, $block:expr) => {{
        let name = $name;
        let details = $details;
        log::info!("RPC: {} invoked ({})", name, details);
        let res = $block;
        match &res {
            Ok(_) => log::info!("RPC: {} succeeded ({})", name, details),
            Err(err) => log::error!("RPC: {} failed ({}): {:?}", name, details, err),
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
    authorize_side,
};
pub use producer::{
    Modules,
    run_battler_service_producer,
    run_battler_service_producer_over_service,
};
