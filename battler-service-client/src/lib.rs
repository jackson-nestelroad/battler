mod client;
mod direct;
mod wamp;

pub use client::{
    BattlerServiceClient,
    battler_service_client_over_direct_service,
    battler_service_client_over_wamp_consumer,
};
pub use direct::DirectBattlerServiceClient;
pub use wamp::SimpleWampBattlerServiceClient;
