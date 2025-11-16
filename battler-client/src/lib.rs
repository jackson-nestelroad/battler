mod client;
pub mod discovery;
pub mod log;
mod manager;
pub mod state;
pub mod state_util;
pub mod ui;

pub use client::{
    BattleClientEvent,
    BattlerClient,
};
pub use manager::BattlerClientManager;
