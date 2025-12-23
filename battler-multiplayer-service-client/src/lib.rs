mod wamp;

pub use battler_multiplayer_service::{
    BattlerMultiplayerServiceClient,
    DirectBattlerMultiplayerServiceClient,
};
pub use wamp::WampBattlerMultiplayerServiceClient;
