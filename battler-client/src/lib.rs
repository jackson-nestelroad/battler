mod client;
mod manager;

pub use client::{
    BattleClientEvent,
    BattleEndedError,
    BattlerClient,
};
pub use manager::BattlerClientManager;
