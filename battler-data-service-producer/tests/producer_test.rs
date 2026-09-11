use std::sync::Arc;

use anyhow::Result;
use battler_data_service_client::{
    BattlerDataServiceClient,
    battler_data_service_client_over_wamp_consumer,
};
use battler_data_service_producer::{
    Modules,
    run_data_service_producer,
};
use battler_data_service_schema::{
    BatchQuery,
    BattlerDataService,
    ResourceOptions,
};
use battler_test_utils::static_local_data_store;
use battler_wamp::{
    peer::new_web_socket_peer,
    router::{
        EmptyConnectionPolicies,
        EmptyPubSubPolicies,
        EmptyRpcPolicies,
        RealmAuthenticationConfig,
        RealmConfig,
        RouterConfig,
        RouterHandle,
        SupportedAuthMethod,
        new_web_socket_router,
    },
};
use battler_wamp_uri::Uri;
use battler_wamprat::peer::{
    PeerConnectionConfig,
    PeerConnectionType,
};
use tokio::{
    sync::{
        broadcast,
        oneshot,
    },
    task::JoinHandle,
};

async fn start_router() -> Result<(RouterHandle, JoinHandle<()>)> {
    let mut config = RouterConfig::default();
    config.realms.push(RealmConfig {
        name: "battler".to_owned(),
        uri: Uri::try_from("com.battler")?,
        authentication: RealmAuthenticationConfig {
            required: false,
            methods: Vec::from_iter([SupportedAuthMethod::Undisputed]),
        },
    });
    let router = new_web_socket_router(
        config,
        Box::new(EmptyConnectionPolicies::default()),
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
    router.start().await
}

struct TestContext {
    router_handle: RouterHandle,
    router_join_handle: JoinHandle<()>,
    producer_stop_tx: Option<broadcast::Sender<()>>,
    producer_join_handle: Option<JoinHandle<Result<()>>>,
}

impl TestContext {
    async fn new() -> Self {
        let (router_handle, router_join_handle) = start_router().await.unwrap();
        Self {
            router_handle,
            router_join_handle,
            producer_stop_tx: None,
            producer_join_handle: None,
        }
    }

    async fn run_producer(&mut self) {
        let (stop_tx, stop_rx) = broadcast::channel(1);
        let data = static_local_data_store();
        let peer = new_web_socket_peer(battler_wamp::peer::PeerConfig {
            name: "data-producer".to_owned(),
            ..Default::default()
        })
        .unwrap();
        let config = battler_wamprat_schema::PeerConfig {
            connection: PeerConnectionConfig::new(PeerConnectionType::Direct(
                self.router_handle.clone(),
            )),
            auth_methods: Vec::default(),
        };
        let (started_tx, started_rx) = oneshot::channel();
        let handle = tokio::spawn(run_data_service_producer(
            data,
            None,
            config,
            peer,
            Modules {
                stop_rx: Some(stop_rx),
                started_tx: Some(started_tx),
            },
        ));
        started_rx.await.unwrap();
        self.producer_stop_tx = Some(stop_tx);
        self.producer_join_handle = Some(handle);
    }

    async fn create_client(&self) -> Box<dyn BattlerDataServiceClient> {
        let peer = new_web_socket_peer(battler_wamp::peer::PeerConfig {
            name: "data-client".to_owned(),
            ..Default::default()
        })
        .unwrap();
        let config = battler_wamprat_schema::PeerConfig {
            connection: PeerConnectionConfig::new(PeerConnectionType::Direct(
                self.router_handle.clone(),
            )),
            auth_methods: Vec::default(),
        };
        let consumer = Arc::new(BattlerDataService::consumer(config, peer).unwrap());
        Box::new(battler_data_service_client_over_wamp_consumer(consumer))
    }

    async fn teardown(self) {
        if let Some(stop_tx) = self.producer_stop_tx {
            stop_tx.send(()).ok();
        }
        if let Some(handle) = self.producer_join_handle {
            handle.await.ok();
        }
        self.router_handle.cancel().ok();
        self.router_join_handle.await.ok();
    }
}

#[tokio::test]
async fn queries_move_ability_item_condition_species_over_wamp() {
    let mut context = TestContext::new().await;
    context.run_producer().await;
    let client = context.create_client().await;

    // Move query (sanitized by default)
    let fly = client
        .get_move("Fly", ResourceOptions::default())
        .await
        .unwrap();
    assert_eq!(fly.name, "Fly");
    assert_eq!(fly.effect, serde_json::Value::Null);
    assert_eq!(fly.condition, serde_json::Value::Null);

    // Move with fxlang included
    let fly_fx = client
        .get_move(
            "Fly",
            ResourceOptions {
                include_fxlang: true,
            },
        )
        .await
        .unwrap();
    assert_ne!(fly_fx.effect, serde_json::Value::Null);
    assert_ne!(fly_fx.condition, serde_json::Value::Null);

    // Move not found returns error
    assert_matches::assert_matches!(
        client
            .get_move("nonexistent_move", ResourceOptions::default())
            .await,
        Err(_)
    );

    // Ability query
    let intimidate = client
        .get_ability("Intimidate", ResourceOptions::default())
        .await
        .unwrap();
    assert_eq!(intimidate.name, "Intimidate");

    // Item query
    let leftovers = client
        .get_item("Leftovers", ResourceOptions::default())
        .await
        .unwrap();
    assert_eq!(leftovers.name, "Leftovers");

    // Condition query
    let sandstorm = client
        .get_condition("Sandstorm", ResourceOptions::default())
        .await
        .unwrap();
    assert_eq!(sandstorm.name, "Sandstorm");

    // Species query
    let pikachu = client
        .get_species("Pikachu", ResourceOptions::default())
        .await
        .unwrap();
    assert_eq!(pikachu.name, "Pikachu");

    // Batch query
    let batch = client
        .batch(BatchQuery {
            moves: vec!["Tackle".to_owned(), "Flamethrower".to_owned()],
            abilities: vec!["Blaze".to_owned()],
            items: vec!["Choice Band".to_owned()],
            conditions: vec!["Rain".to_owned()],
            species: vec!["Charizard".to_owned()],
            options: ResourceOptions::default(),
        })
        .await
        .unwrap();

    assert_matches::assert_matches!(batch.moves.get("Tackle"), Some(Some(_)));
    assert_matches::assert_matches!(batch.moves.get("Flamethrower"), Some(Some(_)));
    assert_matches::assert_matches!(batch.abilities.get("Blaze"), Some(Some(_)));
    assert_matches::assert_matches!(batch.items.get("Choice Band"), Some(Some(_)));
    assert_matches::assert_matches!(batch.conditions.get("Rain"), Some(Some(_)));
    assert_matches::assert_matches!(batch.species.get("Charizard"), Some(Some(_)));

    // Generic resource lookup (e.g. Toxic Spikes -> Move)
    let toxic_spikes = client
        .get_resource(
            "Toxic Spikes",
            battler_data_service_schema::ResourceLookupOptions::default(),
        )
        .await
        .unwrap();
    assert_matches::assert_matches!(toxic_spikes, battler_data_service_schema::ResourceData::Move(data) => {
        assert_eq!(data.name, "Toxic Spikes");
    });

    // Type chart query
    let type_chart = client.get_type_chart().await.unwrap();
    assert!(!type_chart.types.is_empty());
    assert!(type_chart.types.contains_key(&battler_data::Type::Water));

    context.teardown().await;
}
