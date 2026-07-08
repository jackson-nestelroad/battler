use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler::{
    BattleType,
    CoreBattleEngineOptions,
    CoreBattleOptions,
    FieldData,
    FormatData,
    MonData,
    PlayerData,
    SideData,
    TeamData,
};
use battler_multiplayer_service::{
    BattlerMultiplayerService,
    BattlerMultiplayerServiceClient,
    ProposedBattleOptions,
    ProposedBattleResponse,
    ProposedBattleUpdate,
};
use battler_multiplayer_service_producer::{
    Modules,
    MultiplayerBattleAuthorizer,
    run_multiplayer_battler_service_producer_over_service,
};
use battler_multiplayer_service_schema::BattlerMultiplayerServiceConsumer;
use battler_service::{
    BattleServiceOptions,
    BattleState,
};
use battler_service_client::battler_service_client_over_wamp_consumer;
use battler_service_producer::{
    BattleAuthorizer,
    run_battler_service_producer_over_service,
};
use battler_test_utils::static_local_data_store;
use battler_wamp::{
    core::{
        error::{
            BasicError,
            WampError,
        },
        peer_info::{
            ConnectionType,
            PeerInfo,
        },
    },
    peer::{
        Peer,
        WebSocketPeer,
        new_web_socket_peer,
    },
    router::{
        EmptyRpcPolicies,
        PubSubPolicies,
        RealmAuthenticationConfig,
        RealmConfig,
        RouterConfig,
        RouterHandle,
        SessionHandle,
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
use uuid::Uuid;

#[derive(Default)]
struct BattlerPubSubPolicies;

#[async_trait]
impl<S> PubSubPolicies<S> for BattlerPubSubPolicies {
    async fn validate_publication(&self, session: &SessionHandle, _: &Uri) -> Result<()> {
        match session.peer_info().await {
            Some(peer_info) => match peer_info.connection_type {
                ConnectionType::Direct => Ok(()),
                _ => Err(
                    BasicError::NotAllowed("remote connection cannot publish".to_owned()).into(),
                ),
            },
            None => Err(BasicError::Internal("missing peer info during publish".to_owned()).into()),
        }
    }
}

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
        Box::new(BattlerPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
    router.start().await
}

struct Authorizer {
    pub allow_propose: bool,
}

#[async_trait]
impl BattleAuthorizer for Authorizer {
    async fn authorize_new_battle(&self, _: &PeerInfo, _: &CoreBattleOptions) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl MultiplayerBattleAuthorizer for Authorizer {
    async fn authorize_new_proposed_battle(
        &self,
        _: &PeerInfo,
        _: &ProposedBattleOptions,
    ) -> Result<()> {
        if self.allow_propose {
            Ok(())
        } else {
            Err(Error::msg("not allowed"))
        }
    }
}

struct TestContext {
    router_handle: RouterHandle,
    router_join_handle: JoinHandle<()>,
    producer_stop_tx: Option<broadcast::Sender<()>>,
    producer_join_handles: Vec<JoinHandle<Result<()>>>,
}

impl TestContext {
    async fn new() -> Self {
        let (router_handle, router_join_handle) = start_router().await.unwrap();
        Self {
            router_handle,
            router_join_handle,
            producer_stop_tx: None,
            producer_join_handles: Vec::new(),
        }
    }

    async fn run_producers(&mut self, allow_propose: bool) {
        let (stop_tx, stop_rx_1) = broadcast::channel(1);
        let stop_rx_2 = stop_tx.subscribe();
        let data = static_local_data_store();

        // Instantiate core services
        let mut battler_service_local = battler_service::BattlerService::new(data);
        let global_log_rx = battler_service_local.take_global_log_rx().unwrap();
        let battler_service = Arc::new(battler_service_local);

        let battler_service_client = Arc::new(
            battler_service_client::battler_service_client_over_direct_service(
                battler_service.clone(),
            ),
        );
        let multiplayer_service =
            Arc::new(BattlerMultiplayerService::new(data, battler_service_client).await);
        let global_update_rx = multiplayer_service.take_global_update_rx().await.unwrap();

        // Spin up Battle Service Producer
        let battle_peer = create_peer("battle-producer").unwrap();
        let battle_config = battler_wamprat_schema::PeerConfig {
            connection: PeerConnectionConfig::new(PeerConnectionType::Direct(
                self.router_handle.clone(),
            )),
            auth_methods: Vec::default(),
        };
        let (started_tx_1, started_rx_1) = oneshot::channel();
        let handle_1 = tokio::spawn(run_battler_service_producer_over_service(
            battler_service,
            global_log_rx,
            CoreBattleEngineOptions {
                log_time: false,
                ..Default::default()
            },
            battle_config,
            battle_peer,
            battler_service_producer::Modules {
                authorizer: Box::new(Authorizer { allow_propose }),
                stop_rx: Some(stop_rx_1),
                started_tx: Some(started_tx_1),
            },
        ));

        // Spin up Multiplayer Service Producer
        let mp_peer = create_peer("mp-producer").unwrap();
        let mp_config = battler_wamprat_schema::PeerConfig {
            connection: PeerConnectionConfig::new(PeerConnectionType::Direct(
                self.router_handle.clone(),
            )),
            auth_methods: Vec::default(),
        };
        let (started_tx_2, started_rx_2) = oneshot::channel();
        let handle_2 = tokio::spawn(run_multiplayer_battler_service_producer_over_service(
            multiplayer_service,
            global_update_rx,
            mp_config,
            mp_peer,
            Modules {
                authorizer: Box::new(Authorizer { allow_propose }),
                stop_rx: Some(stop_rx_2),
                started_tx: Some(started_tx_2),
            },
        ));

        started_rx_1.await.unwrap();
        started_rx_2.await.unwrap();

        self.producer_stop_tx = Some(stop_tx);
        self.producer_join_handles = Vec::from_iter([handle_1, handle_2]);
    }

    async fn teardown(self) {
        if let Some(stop_tx) = self.producer_stop_tx {
            stop_tx.send(()).ok();
        }
        for handle in self.producer_join_handles {
            handle.await.ok();
        }
        self.router_handle.cancel().ok();
        self.router_join_handle.await.ok();
    }
}

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    let config = battler_wamp::peer::PeerConfig {
        name: format!(
            "{name}-{}",
            std::thread::current().name().unwrap_or("NO_THREAD_NAME")
        ),
        ..Default::default()
    };
    new_web_socket_peer(config)
}

async fn start_multiplayer_consumer<S>(
    name: &str,
    connection_type: PeerConnectionType,
    peer: Peer<S>,
) -> Result<BattlerMultiplayerServiceConsumer<S>>
where
    S: Send + 'static,
{
    let mut connection = PeerConnectionConfig::new(connection_type);
    connection.reconnect_delay = Duration::from_millis(50);
    connection.max_consecutive_failures = 100;
    let consumer = battler_multiplayer_service_schema::BattlerMultiplayerService::consumer(
        battler_wamprat_schema::PeerConfig {
            connection,
            auth_methods: Vec::from_iter([battler_wamp::peer::SupportedAuthMethod::Undisputed {
                id: name.to_owned(),
                role: "user".to_owned(),
            }]),
        },
        peer,
    )?;
    consumer.wait_until_ready().await?;
    Ok(consumer)
}

async fn start_battle_consumer<S>(
    name: &str,
    connection_type: PeerConnectionType,
    peer: Peer<S>,
) -> Result<battler_service_schema::BattlerServiceConsumer<S>>
where
    S: Send + 'static,
{
    let mut connection = PeerConnectionConfig::new(connection_type);
    connection.reconnect_delay = Duration::from_millis(50);
    connection.max_consecutive_failures = 100;
    let consumer = battler_service_schema::BattlerService::consumer(
        battler_wamprat_schema::PeerConfig {
            connection,
            auth_methods: Vec::from_iter([battler_wamp::peer::SupportedAuthMethod::Undisputed {
                id: name.to_owned(),
                role: "user".to_owned(),
            }]),
        },
        peer,
    )?;
    consumer.wait_until_ready().await?;
    Ok(consumer)
}

fn proposed_battle_options() -> ProposedBattleOptions {
    ProposedBattleOptions {
        battle_options: CoreBattleOptions {
            seed: Some(0),
            format: FormatData {
                battle_type: BattleType::Singles,
                ..Default::default()
            },
            field: FieldData::default(),
            side_1: SideData {
                name: "Side 1".to_owned(),
                players: Vec::from_iter([PlayerData {
                    id: "player_1".to_owned(),
                    name: "Player 1".to_owned(),
                    team: TeamData {
                        members: Vec::from_iter([MonData {
                            name: "Pikachu".to_owned(),
                            species: "Pikachu".to_owned(),
                            ability: "Static".to_owned(),
                            moves: Vec::from_iter(["Tackle".to_owned()]),
                            level: 5,
                            ..Default::default()
                        }]),
                        ..Default::default()
                    },
                    ..Default::default()
                }]),
            },
            side_2: SideData {
                name: "Side 2".to_owned(),
                players: Vec::from_iter([PlayerData {
                    id: "player_2".to_owned(),
                    name: "Player 2".to_owned(),
                    team: TeamData {
                        members: Vec::from_iter([MonData {
                            name: "Meowth".to_owned(),
                            species: "Meowth".to_owned(),
                            ability: "Pickup".to_owned(),
                            moves: Vec::from_iter(["Scratch".to_owned()]),
                            level: 5,
                            ..Default::default()
                        }]),
                        ..Default::default()
                    },
                    ..Default::default()
                }]),
            },
        },
        service_options: BattleServiceOptions {
            creator: "player_1".to_owned(),
            ..Default::default()
        },
        timeout: Duration::from_secs(30),
    }
}

async fn read_all_updates_stopping_at_battle_id_or_timeout(
    update_rx: &mut broadcast::Receiver<ProposedBattleUpdate>,
    timeout: Duration,
) -> Option<Uuid> {
    let deadline = Instant::now() + timeout;
    loop {
        tokio::select! {
            update = update_rx.recv() => {
                if let Ok(update) = update {
                    if let Some(battle) = update.proposed_battle.battle {
                        return Some(battle);
                    }
                }
            }
            _ = tokio::time::sleep_until(deadline.into()) => break,
        }
    }
    None
}

#[tokio::test(flavor = "multi_thread")]
async fn proposes_and_starts_battle_lifecycle() {
    battler_test_utils::collect_logs();

    let mut context = TestContext::new().await;
    context.run_producers(true).await;

    // Create client connection for Player 1
    let peer_1 = create_peer("player_1").unwrap();
    let mp_consumer_1 = start_multiplayer_consumer(
        "player_1",
        PeerConnectionType::Direct(context.router_handle.clone()),
        peer_1,
    )
    .await
    .unwrap();

    let mp_client_1 = battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(
        Arc::new(mp_consumer_1),
    );

    // Create client connection for Player 2
    let peer_2 = create_peer("player_2").unwrap();
    let mp_consumer_2 = start_multiplayer_consumer(
        "player_2",
        PeerConnectionType::Direct(context.router_handle.clone()),
        peer_2,
    )
    .await
    .unwrap();

    let mp_client_2 = battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(
        Arc::new(mp_consumer_2),
    );

    // Subscribe both players to matchmaking updates
    let mut update_rx_1 = mp_client_1
        .proposed_battle_updates("player_1")
        .await
        .unwrap();

    // Player 1 proposes a battle
    let proposed = mp_client_1
        .propose_battle(proposed_battle_options())
        .await
        .unwrap();

    // Player 2 accepts the proposed battle
    let accepted = mp_client_2
        .respond_to_proposed_battle(
            proposed.uuid,
            "player_2",
            ProposedBattleResponse { accept: true },
        )
        .await
        .unwrap();

    assert_eq!(
        accepted
            .sides
            .iter()
            .flat_map(|side| side.players.iter())
            .filter_map(|p| p.status.clone())
            .collect::<Vec<_>>(),
        [
            battler_multiplayer_service::PlayerStatus::Accepted,
            battler_multiplayer_service::PlayerStatus::Accepted
        ]
    );

    // Wait until the battle UUID is generated (indicating the matchmaking loop started it)
    let battle_uuid =
        read_all_updates_stopping_at_battle_id_or_timeout(&mut update_rx_1, Duration::from_secs(3))
            .await
            .expect("should have created battle");

    // Connect to the battle service via WAMP and verify the active battle exists
    let battle_peer = create_peer("battle-client").unwrap();
    let battle_consumer = start_battle_consumer(
        "player_1",
        PeerConnectionType::Direct(context.router_handle.clone()),
        battle_peer,
    )
    .await
    .unwrap();

    let battle_client = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer));
    let battle = battle_client.battle(battle_uuid).await.unwrap();

    assert_eq!(battle.state, BattleState::Active);

    context.teardown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn enforces_authorization() {
    battler_test_utils::collect_logs();

    let mut context = TestContext::new().await;
    // Disable proposals in authorizer
    context.run_producers(false).await;

    let peer = create_peer("player_1").unwrap();
    let mp_consumer = start_multiplayer_consumer(
        "player_1",
        PeerConnectionType::Direct(context.router_handle.clone()),
        peer,
    )
    .await
    .unwrap();

    let mp_client = battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(
        Arc::new(mp_consumer),
    );

    // Try proposing and verify authorization failure
    let result = mp_client.propose_battle(proposed_battle_options()).await;

    assert_matches::assert_matches!(result, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<WampError>(), Ok(err) => {
            assert_eq!(err.to_string(), "not allowed");
        });
    });

    context.teardown().await;
}
