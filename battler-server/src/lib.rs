use std::{
    net::IpAddr,
    sync::Arc,
    time::Duration,
};

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler::{
    CoreBattleEngineOptions,
    CoreBattleOptions,
};
use battler_local_data::LocalDataStore;
use battler_multiplayer_service::{
    AiPlayerOptions,
    AiPlayerType,
    AiPlayers,
    BattlerMultiplayerService,
    ProposedBattleOptions,
    RandomOptions,
};
use battler_multiplayer_service_producer::MultiplayerBattleAuthorizer;
use battler_service::Battle;
use battler_service_producer::{
    BattleAuthorizer,
    BattleOperation,
    PlayerOperation,
    authorize_battle_owner,
    authorize_player,
    authorize_side,
};
use battler_wamp::{
    core::{
        error::BasicError,
        peer_info::{
            ConnectionType,
            PeerInfo,
        },
    },
    peer::new_web_socket_peer,
    router::{
        ConnectionPolicies,
        PubSubPolicies,
        RealmAuthenticationConfig,
        RealmConfig,
        RouterConfig,
        RouterHandle,
        RpcPolicies,
        SessionHandle,
        SupportedAuthMethod,
        new_web_socket_router,
    },
};
use battler_wamp_uri::{
    Uri,
    WildcardUri,
};
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

#[derive(Default)]
struct BattlerConnectionPolicies;

#[async_trait]
impl<S> ConnectionPolicies<S> for BattlerConnectionPolicies {
    async fn validate_connection(
        &self,
        _session: &SessionHandle,
        peer_info: &PeerInfo,
    ) -> Result<()> {
        if let ConnectionType::Direct = peer_info.connection_type {
            return Ok(());
        }
        let id = &peer_info.identity.id;
        if id.starts_with("ai-") {
            return Err(BasicError::PermissionDenied(
                "player id prefix is reserved for AI players".to_owned(),
            )
            .into());
        }
        Ok(())
    }
}

#[derive(Default)]
struct BattlerPubSubPolicies;

#[async_trait]
impl<S> PubSubPolicies<S> for BattlerPubSubPolicies {
    async fn validate_publication(&self, session: &SessionHandle, uri: &Uri) -> Result<()> {
        let uri_str = uri.to_string();
        let is_battler_topic = uri_str.starts_with("com.battler.battler_service")
            || uri_str.starts_with("com.battler.battler_multiplayer_service");

        if !is_battler_topic {
            return Ok(());
        }
        match session.peer_info().await {
            Some(peer_info) => match peer_info.connection_type {
                ConnectionType::Direct => Ok(()),
                _ => Err(BasicError::NotAllowed(
                    "remote connection cannot publish to battler service topics".to_owned(),
                )
                .into()),
            },
            None => Err(BasicError::Internal("missing peer info during publish".to_owned()).into()),
        }
    }
}

#[derive(Default)]
struct BattlerRpcPolicies;

#[async_trait]
impl<S> RpcPolicies<S> for BattlerRpcPolicies {
    async fn validate_registration(
        &self,
        session: &SessionHandle,
        procedure: &WildcardUri,
    ) -> Result<()> {
        let proc_str = procedure.to_string();
        let is_battler_service = proc_str.starts_with("com.battler.battler_service")
            || proc_str.starts_with("com.battler.battler_multiplayer_service");

        if !is_battler_service {
            return Ok(());
        }

        match session.peer_info().await {
            Some(peer_info) => match peer_info.connection_type {
                ConnectionType::Direct => Ok(()),
                _ => Err(Error::msg(
                    "remote connection is not allowed to register procedures on battler service namespaces",
                )),
            },
            None => Err(Error::msg("missing peer info during registration")),
        }
    }
}

struct ServerAuthorizer;

#[async_trait]
impl BattleAuthorizer for ServerAuthorizer {
    async fn authorize_new_battle(
        &self,
        peer_info: &PeerInfo,
        _: &CoreBattleOptions,
    ) -> Result<()> {
        match peer_info.connection_type {
            ConnectionType::Direct => Ok(()),
            _ => Err(Error::msg(
                "remote connection is not allowed to create a battle directly",
            )),
        }
    }

    async fn authorize_battle_operation(
        &self,
        peer_info: &PeerInfo,
        battle: &Battle,
        _operation: BattleOperation,
    ) -> Result<()> {
        if let ConnectionType::Direct = peer_info.connection_type {
            return Ok(());
        }
        authorize_battle_owner(peer_info, battle)
    }

    async fn authorize_player_operation(
        &self,
        peer_info: &PeerInfo,
        player: &str,
        _operation: PlayerOperation,
    ) -> Result<()> {
        if let ConnectionType::Direct = peer_info.connection_type {
            return Ok(());
        }
        authorize_player(peer_info, player)
    }

    async fn authorize_log_access(
        &self,
        peer_info: &PeerInfo,
        battle: &Battle,
        side: Option<usize>,
    ) -> Result<()> {
        if let ConnectionType::Direct = peer_info.connection_type {
            return Ok(());
        }
        authorize_side(peer_info, battle, side)
    }
}

#[async_trait]
impl MultiplayerBattleAuthorizer for ServerAuthorizer {
    async fn authorize_new_proposed_battle(
        &self,
        peer_info: &PeerInfo,
        options: &ProposedBattleOptions,
    ) -> Result<()> {
        if let ConnectionType::Direct = peer_info.connection_type {
            return Ok(());
        }

        let caller_id = &peer_info.identity.id;
        let creator_id = &options.service_options.creator;
        if caller_id.is_empty() {
            return Err(Error::msg("unauthenticated caller cannot propose a battle"));
        }
        if caller_id != creator_id {
            return Err(Error::msg(format!(
                "caller '{caller_id}' cannot propose a battle on behalf of creator '{creator_id}'"
            )));
        }
        Ok(())
    }
}

pub struct ServerConfig {
    pub address: IpAddr,
    pub port: u16,
    pub data_dir: String,
    pub realm_name: String,
    pub realm_uri: String,
}

pub struct ServerHandle {
    pub router_handle: RouterHandle,
    pub router_join_handle: JoinHandle<()>,
    pub battle_producer_handle: JoinHandle<Result<()>>,
    pub multiplayer_producer_handle: JoinHandle<Result<()>>,
    pub stop_tx: broadcast::Sender<()>,
}

impl ServerHandle {
    pub async fn shutdown(self) -> Result<()> {
        let _ = self.stop_tx.send(());
        let _ = self.battle_producer_handle.await;
        let _ = self.multiplayer_producer_handle.await;
        let _ = self.router_handle.cancel();
        let _ = self.router_join_handle.await;
        Ok(())
    }
}

pub async fn start_server(config: ServerConfig) -> Result<ServerHandle> {
    // 1. Initialize local data store from disk (using Box::leak for static lifetime)
    let data_store = Box::leak(Box::new(LocalDataStore::new(config.data_dir)?));

    // 2. Setup WAMP router config
    let mut router_config = RouterConfig::default();
    router_config.address = config.address;
    router_config.port = config.port;
    router_config.realms.push(RealmConfig {
        name: config.realm_name,
        uri: Uri::try_from(config.realm_uri.as_str())?,
        authentication: RealmAuthenticationConfig {
            required: false,
            methods: Vec::from_iter([SupportedAuthMethod::Undisputed]),
        },
    });

    let router = new_web_socket_router(
        router_config,
        Box::new(BattlerConnectionPolicies::default()),
        Box::new(BattlerPubSubPolicies::default()),
        Box::new(BattlerRpcPolicies::default()),
    )?;
    let (router_handle, router_join_handle) = router.start().await?;

    // 3. Setup service structures and channels
    let (stop_tx, _) = broadcast::channel(1);
    let (started_tx_1, started_rx_1) = oneshot::channel();
    let (started_tx_2, started_rx_2) = oneshot::channel();

    // Battle Service setup
    let mut battler_service_local = battler_service::BattlerService::new(data_store);
    let global_log_rx = battler_service_local.take_global_log_rx().unwrap();
    let battler_service = Arc::new(battler_service_local);

    // Spawn housekeeping task for finished battles
    let battler_service_cleanup = battler_service.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            if let Err(err) = battler_service_cleanup
                .clean_up_finished_battles(Duration::from_secs(60))
                .await
            {
                log::error!("Error cleaning up finished battles: {err:?}");
            }
        }
    });

    // Multiplayer Service setup
    let battler_service_client = Arc::new(
        battler_service_client::battler_service_client_over_direct_service(battler_service.clone()),
    );
    let multiplayer_service =
        Arc::new(BattlerMultiplayerService::new(data_store, battler_service_client).await);
    let global_update_rx = multiplayer_service.take_global_update_rx().await.unwrap();

    // Register AI players.
    let ai_ids = (1..=10)
        .map(|i| format!("ai-random-{i}"))
        .collect::<HashSet<_>>();
    multiplayer_service
        .clone()
        .create_ai_players(AiPlayers {
            players: HashMap::from_iter([(
                "ai-random".to_owned(),
                AiPlayerOptions {
                    ai_type: AiPlayerType::Random(RandomOptions::default()),
                    players: ai_ids,
                },
            )]),
        })
        .await?;

    // 4. Spin up Battle Service Producer
    let battle_peer = new_web_socket_peer(battler_wamp::peer::PeerConfig {
        name: "battle-producer".to_owned(),
        ..Default::default()
    })?;
    let battle_config = battler_wamprat_schema::PeerConfig {
        connection: PeerConnectionConfig::new(PeerConnectionType::Direct(router_handle.clone())),
        auth_methods: Vec::default(),
    };
    let stop_rx_1 = stop_tx.subscribe();
    let battle_producer_handle = tokio::spawn(
        battler_service_producer::run_battler_service_producer_over_service(
            battler_service,
            global_log_rx,
            CoreBattleEngineOptions::default(),
            battle_config,
            battle_peer,
            battler_service_producer::Modules {
                authorizer: Box::new(ServerAuthorizer),
                stop_rx: Some(stop_rx_1),
                started_tx: Some(started_tx_1),
            },
        ),
    );

    // 5. Spin up Multiplayer Service Producer
    let multiplayer_peer = new_web_socket_peer(battler_wamp::peer::PeerConfig {
        name: "mp-producer".to_owned(),
        ..Default::default()
    })?;
    let multiplayer_config = battler_wamprat_schema::PeerConfig {
        connection: PeerConnectionConfig::new(PeerConnectionType::Direct(router_handle.clone())),
        auth_methods: Vec::default(),
    };
    let stop_rx_2 = stop_tx.subscribe();
    let multiplayer_producer_handle = tokio::spawn(
        battler_multiplayer_service_producer::run_multiplayer_battler_service_producer_over_service(
            multiplayer_service,
            global_update_rx,
            multiplayer_config,
            multiplayer_peer,
            battler_multiplayer_service_producer::Modules {
                authorizer: Box::new(ServerAuthorizer),
                stop_rx: Some(stop_rx_2),
                started_tx: Some(started_tx_2),
            },
        ),
    );

    // Wait until both producers are connected and active
    started_rx_1.await?;
    started_rx_2.await?;

    Ok(ServerHandle {
        router_handle,
        router_join_handle,
        battle_producer_handle,
        multiplayer_producer_handle,
        stop_tx,
    })
}
