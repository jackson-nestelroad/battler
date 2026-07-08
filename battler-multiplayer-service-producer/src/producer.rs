use std::{
    pin::Pin,
    sync::Arc,
};

use anyhow::{
    Error,
    Result,
};
use battler::{
    CoreBattleEngineOptions,
    DataStore,
    DataStoreByName,
};
use battler_multiplayer_service::{
    BattlerMultiplayerService,
    ProposedBattleUpdate,
};
use futures_util::Future;
use tokio::sync::{
    broadcast,
    mpsc,
    oneshot,
};

use crate::{
    MultiplayerBattleAuthorizer,
    handlers,
};

pub struct Modules {
    pub authorizer: Box<dyn MultiplayerBattleAuthorizer>,
    pub stop_rx: Option<broadcast::Receiver<()>>,
    pub started_tx: Option<oneshot::Sender<()>>,
}

pub async fn run_multiplayer_battler_service_producer<'d, S>(
    data: &'d dyn DataStoreByName,
    engine_options: CoreBattleEngineOptions,
    peer_config: battler_wamprat_schema::PeerConfig,
    peer: battler_wamp::peer::Peer<S>,
    modules: Modules,
) -> Result<()>
where
    S: Send + 'static,
{
    // SAFETY: The `BattlerMultiplayerService` instance, which borrows `data`, is dropped at the end
    // of this function.
    let data = unsafe {
        std::mem::transmute::<&'d dyn DataStoreByName, &'static dyn DataStoreByName>(data)
    };
    let battler_service = Arc::new(battler_service::BattlerService::new(data as &dyn DataStore));
    let mut battler_service_client =
        battler_service_client::DirectBattlerServiceClient::new(battler_service.clone());
    *battler_service_client.engine_options_mut() = engine_options;
    let battler_service_client: Arc<Box<dyn battler_service_client::BattlerServiceClient>> =
        Arc::new(Box::new(battler_service_client));
    let service = BattlerMultiplayerService::new(data, battler_service_client).await;
    let global_update_rx = service
        .take_global_update_rx()
        .await
        .ok_or_else(|| Error::msg("expected global update receiver"))?;
    run_multiplayer_battler_service_producer_over_service(
        Arc::new(service),
        global_update_rx,
        peer_config,
        peer,
        modules,
    )
    .await
}

pub async fn run_multiplayer_battler_service_producer_over_service<S>(
    service: Arc<BattlerMultiplayerService<'static>>,
    global_update_rx: mpsc::UnboundedReceiver<ProposedBattleUpdate>,
    peer_config: battler_wamprat_schema::PeerConfig,
    peer: battler_wamp::peer::Peer<S>,
    modules: Modules,
) -> Result<()>
where
    S: Send + 'static,
{
    let mut builder =
        battler_multiplayer_service_schema::BattlerMultiplayerService::producer_builder(
            peer_config.clone(),
        );
    let authorizer = Arc::new(modules.authorizer);

    builder.register_propose_battle(handlers::propose_battle::Handler {
        service: service.clone(),
        authorizer: authorizer.clone(),
    })?;
    builder.register_proposed_battle(handlers::proposed_battle::Handler {
        service: service.clone(),
    })?;
    builder.register_respond_to_proposed_battle(handlers::respond_to_proposed_battle::Handler {
        service: service.clone(),
        authorizer: authorizer.clone(),
    })?;
    builder.register_proposed_battles_for_player(
        handlers::proposed_battles_for_player::Handler {
            service: service.clone(),
            authorizer: authorizer.clone(),
        },
    )?;

    let producer = builder.start(peer)?;

    if let Some(started_tx) = modules.started_tx {
        producer.wait_until_ready().await?;
        started_tx
            .send(())
            .map_err(|_| Error::msg("writing to started_tx failed"))?;
    }

    run_multiplayer_battler_service_producer_internal(producer, modules.stop_rx, global_update_rx)
        .await?;

    Ok(())
}

async fn run_multiplayer_battler_service_producer_internal<S>(
    producer: battler_multiplayer_service_schema::BattlerMultiplayerServiceProducer<S>,
    mut stop_rx: Option<broadcast::Receiver<()>>,
    mut global_update_rx: mpsc::UnboundedReceiver<ProposedBattleUpdate>,
) -> Result<()>
where
    S: Send + 'static,
{
    loop {
        let stop_recv: Pin<
            Box<dyn Future<Output = Result<(), broadcast::error::RecvError>> + Send>,
        > = match &mut stop_rx {
            Some(stop_rx) => Box::pin(stop_rx.recv()),
            None => Box::pin(futures_util::future::pending()),
        };
        tokio::select! {
            update = global_update_rx.recv() => {
                publish_update(
                    &producer,
                    update.ok_or_else(|| Error::msg("global update channel unexpectedly closed"))?,
                ).await?;
            },
            _ = stop_recv => {
                producer.stop().await?;
                break;
            },
        }
    }
    Ok(())
}

async fn publish_update<S>(
    producer: &battler_multiplayer_service_schema::BattlerMultiplayerServiceProducer<S>,
    update: ProposedBattleUpdate,
) -> Result<()>
where
    S: Send + 'static,
{
    let event = battler_multiplayer_service_schema::ProposedBattleUpdateEvent(
        battler_multiplayer_service_schema::ProposedBattleUpdate {
            proposed_battle_update_json: serde_json::to_string(&update)?,
        },
    );
    let players = update
        .proposed_battle
        .sides
        .iter()
        .flat_map(|side| side.players.iter())
        .map(|player| player.id.clone())
        .collect::<Vec<_>>();
    for player in players {
        let pattern = battler_multiplayer_service_schema::ProposedBattleUpdatesPattern {
            player: player.clone(),
        };
        producer
            .publish_proposed_battle_updates(
                pattern,
                event.clone(),
                battler_wamprat::peer::PublishOptions::default(),
            )
            .await?;
    }
    Ok(())
}
