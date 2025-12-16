use std::sync::Arc;

use anyhow::{
    Error,
    Result,
};
use battler::{
    CoreBattleEngineOptions,
    DataStore,
};
use battler_service::{
    BattlerService,
    GlobalLogEntry,
};
use tokio::sync::{
    broadcast,
    mpsc,
};
use uuid::Uuid;

use crate::handlers::{
    battle,
    battles,
    battles_for_player,
    create,
    delete,
    full_log,
    last_log_entry,
    make_choice,
    player_data,
    request,
    start,
    update_team,
    validate_player,
};

fn uuid_for_uri(uuid: &Uuid) -> String {
    uuid.simple()
        .encode_lower(&mut Uuid::encode_buffer())
        .to_owned()
}

pub struct Modules {
    pub create_authorizer: Box<dyn create::Authorizer>,
    pub start_authorizer: Box<dyn start::Authorizer>,
    pub delete_authorizer: Box<dyn delete::Authorizer>,
}

pub async fn run_battler_service_producer<'d, S>(
    data: &'d dyn DataStore,
    engine_options: CoreBattleEngineOptions,
    peer_config: battler_wamprat_schema::PeerConfig,
    peer: battler_wamp::peer::Peer<S>,
    stop_rx: broadcast::Receiver<()>,
    modules: Modules,
) -> Result<()>
where
    S: Send + 'static,
{
    // SAFETY: The `BattlerService` instance, which borrows `data`, is dropped at the end of this
    // function.
    let data = unsafe { std::mem::transmute::<&'d dyn DataStore, &'static dyn DataStore>(data) };
    let mut service = BattlerService::new(data);
    let global_log_rx = service
        .take_global_log_rx()
        .ok_or_else(|| Error::msg("expected global log receiver"))?;
    let mut builder = battler_service_schema::BattlerService::producer_builder(peer_config.clone());
    let service = Arc::new(service);

    builder.register_create(create::Handler {
        service: service.clone(),
        engine_options,
        authorizer: modules.create_authorizer,
    })?;
    builder.register_battles(battles::Handler {
        service: service.clone(),
    })?;
    builder.register_battles_for_player(battles_for_player::Handler {
        service: service.clone(),
    })?;
    builder.register_battle(battle::Handler {
        service: service.clone(),
    })?;
    builder.register_update_team(update_team::Handler {
        service: service.clone(),
    })?;
    builder.register_validate_player(validate_player::Handler {
        service: service.clone(),
    })?;
    builder.register_start(start::Handler {
        service: service.clone(),
        authorizer: modules.start_authorizer,
    })?;
    builder.register_player_data(player_data::Handler {
        service: service.clone(),
    })?;
    builder.register_request(request::Handler {
        service: service.clone(),
    })?;
    builder.register_make_choice(make_choice::Handler {
        service: service.clone(),
    })?;
    builder.register_delete(delete::Handler {
        service: service.clone(),
        authorizer: modules.delete_authorizer,
    })?;
    builder.register_full_log(full_log::Handler {
        service: service.clone(),
    })?;
    builder.register_last_log_entry(last_log_entry::Handler {
        service: service.clone(),
    })?;

    let producer = builder.start(peer)?;
    run_battler_service_producer_internal(producer, stop_rx, global_log_rx).await?;

    Arc::try_unwrap(service).unwrap_or_else(|_| {
        panic!("battler service has additional references after producer was dropped")
    });

    Ok(())
}

async fn run_battler_service_producer_internal<'d, S>(
    producer: battler_service_schema::BattlerServiceProducer<S>,
    mut stop_rx: broadcast::Receiver<()>,
    mut global_log_rx: mpsc::UnboundedReceiver<GlobalLogEntry>,
) -> Result<()>
where
    S: Send + 'static,
{
    loop {
        tokio::select! {
            log = global_log_rx.recv() => {
                publish_log_entry(
                    &producer,
                    log.ok_or_else(|| Error::msg("global log channel unexpectedly closed"))?,
                ).await?;

            },
            _ = stop_rx.recv() => {
                producer.stop().await?;
                break;
            },
        }
    }
    Ok(())
}

async fn publish_log_entry<S>(
    producer: &battler_service_schema::BattlerServiceProducer<S>,
    global_log_entry: GlobalLogEntry,
) -> Result<()>
where
    S: Send + 'static,
{
    let log_pattern = battler_service_schema::LogPattern(
        uuid_for_uri(&global_log_entry.battle),
        match global_log_entry.side {
            Some(side) => battler_service_schema::LogSelector::Side(side),
            None => battler_service_schema::LogSelector::Public,
        },
    );
    let log_event = battler_service_schema::LogEvent(battler_service_schema::LogEntry {
        index: global_log_entry.entry.index as battler_wamp_values::Integer,
        content: global_log_entry.entry.content,
    });
    producer
        .publish_log(
            log_pattern,
            log_event,
            battler_wamprat::peer::PublishOptions::default(),
        )
        .await
}
