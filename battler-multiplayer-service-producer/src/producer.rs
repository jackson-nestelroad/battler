use anyhow::Result;
use battler::{
    CoreBattleEngineOptions,
    DataStore,
};
use tokio::sync::{
    broadcast,
    oneshot,
};
use uuid::Uuid;

use crate::MultiplayerBattleAuthorizer;

fn uuid_for_uri(uuid: &Uuid) -> String {
    uuid.simple()
        .encode_lower(&mut Uuid::encode_buffer())
        .to_owned()
}

pub struct Modules {
    pub authorizer: Box<dyn MultiplayerBattleAuthorizer>,
    pub stop_rx: Option<broadcast::Receiver<()>>,
    pub started_tx: Option<oneshot::Sender<()>>,
}

pub async fn run_multiplayer_battler_service_producer<'d, S>(
    data: &'d dyn DataStore,
    engine_options: CoreBattleEngineOptions,
    peer_config: battler_wamprat_schema::PeerConfig,
    peer: battler_wamp::peer::Peer<S>,
    modules: Modules,
) -> Result<()>
where
    S: Send + 'static,
{
    todo!()
}
