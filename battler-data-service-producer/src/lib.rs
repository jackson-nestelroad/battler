pub mod handlers;

use std::sync::Arc;

use anyhow::{
    Error,
    Result,
};
use battler_data::DataStore;
use battler_data_service::BattlerDataService;
use tokio::sync::{
    broadcast,
    oneshot,
};

/// Modules passed to the data service producer.
pub struct Modules {
    pub stop_rx: Option<broadcast::Receiver<()>>,
    pub started_tx: Option<oneshot::Sender<()>>,
}

/// Runs the battler data service producer over a data store.
pub async fn run_data_service_producer<'d, S>(
    data: &'d dyn DataStore,
    descriptions: Option<&'d dyn battler_data_service_schema::DescriptionStore>,
    peer_config: battler_wamprat_schema::PeerConfig,
    peer: battler_wamp::peer::Peer<S>,
    modules: Modules,
) -> Result<()>
where
    S: Send + 'static,
{
    // SAFETY: The `BattlerDataService` instance, which borrows `data` and `descriptions`, is
    // dropped at the end of this function.
    let data = unsafe { std::mem::transmute::<&'d dyn DataStore, &'static dyn DataStore>(data) };
    let descriptions = unsafe {
        std::mem::transmute::<
            Option<&'d dyn battler_data_service_schema::DescriptionStore>,
            Option<&'static dyn battler_data_service_schema::DescriptionStore>,
        >(descriptions)
    };
    let mut service = BattlerDataService::new(data);
    if let Some(descriptions) = descriptions {
        service.set_descriptions(descriptions);
    }
    let service = Arc::new(service);
    run_data_service_producer_over_service(service, peer_config, peer, modules).await
}

/// Runs the battler data service producer over an existing [`BattlerDataService`].
pub async fn run_data_service_producer_over_service<S>(
    service: Arc<BattlerDataService<'static>>,
    peer_config: battler_wamprat_schema::PeerConfig,
    peer: battler_wamp::peer::Peer<S>,
    modules: Modules,
) -> Result<()>
where
    S: Send + 'static,
{
    let mut builder =
        battler_data_service_schema::BattlerDataService::producer_builder(peer_config);

    builder.register_move(handlers::MoveHandler {
        service: service.clone(),
    })?;
    builder.register_ability(handlers::AbilityHandler {
        service: service.clone(),
    })?;
    builder.register_item(handlers::ItemHandler {
        service: service.clone(),
    })?;
    builder.register_condition(handlers::ConditionHandler {
        service: service.clone(),
    })?;
    builder.register_species(handlers::SpeciesHandler {
        service: service.clone(),
    })?;
    builder.register_resource(handlers::ResourceHandler {
        service: service.clone(),
    })?;
    builder.register_batch(handlers::BatchHandler {
        service: service.clone(),
    })?;
    builder.register_type_chart(handlers::TypeChartHandler {
        service: service.clone(),
    })?;

    let producer = builder.start(peer)?;

    if let Some(started_tx) = modules.started_tx {
        producer.wait_until_ready().await?;
        started_tx
            .send(())
            .map_err(|_| Error::msg("writing to started_tx failed"))?;
    }

    if let Some(mut stop_rx) = modules.stop_rx {
        let _ = stop_rx.recv().await;
    } else {
        std::future::pending::<()>().await;
    }

    Ok(())
}
