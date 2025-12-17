use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler::{
    CoreBattleEngineOptions,
    CoreBattleOptions,
};
use battler_service::{
    Battle,
    BattleServiceOptions,
};
use battler_service_producer::{
    BattleAuthorizer,
    Modules,
    run_battler_service_producer,
};
use battler_service_schema::{
    BattlerService,
    BattlerServiceConsumer,
    BattlesInput,
    BattlesInputArgs,
    CreateInput,
    CreateInputArgs,
};
use battler_test_utils::static_local_data_store;
use battler_wamp::{
    core::{
        error::WampError,
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
    CallOptions,
    PeerConnectionConfig,
    PeerConnectionType,
};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
};

async fn start_router_with_config(
    mut config: RouterConfig,
) -> Result<(RouterHandle, JoinHandle<()>)> {
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
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
    router.start().await
}

async fn start_router() -> Result<(RouterHandle, JoinHandle<()>)> {
    start_router_with_config(RouterConfig::default()).await
}

struct Authorizer;

impl Authorizer {
    fn authorize(&self, peer_info: &PeerInfo) -> Result<()> {
        match peer_info.connection_type {
            ConnectionType::Direct => Ok(()),
            _ => Err(Error::msg("not allowed")),
        }
    }
}

#[async_trait]
impl BattleAuthorizer for Authorizer {
    async fn authorize_new_battle(
        &self,
        peer_info: &PeerInfo,
        _: &CoreBattleOptions,
    ) -> Result<()> {
        self.authorize(peer_info)
    }

    async fn authorize_battle_management(&self, peer_info: &PeerInfo, _: &Battle) -> Result<()> {
        self.authorize(peer_info)
    }
}

async fn run_producer(router: RouterHandle) -> Result<()> {
    let data = static_local_data_store();
    let peer = new_web_socket_peer(battler_wamp::peer::PeerConfig::default())?;
    let config = battler_wamprat_schema::PeerConfig {
        connection: PeerConnectionConfig::new(PeerConnectionType::Direct(router)),
        auth_methods: Vec::default(),
    };
    let (started_tx, started_rx) = oneshot::channel();
    tokio::spawn(run_battler_service_producer(
        data,
        CoreBattleEngineOptions::default(),
        config,
        peer,
        Modules {
            authorizer: Box::new(Authorizer),
            stop_rx: None,
            started_tx: Some(started_tx),
        },
    ));
    started_rx.await?;
    Ok(())
}

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    let config = battler_wamp::peer::PeerConfig {
        name: name.to_owned(),
        ..Default::default()
    };
    new_web_socket_peer(config)
}

async fn start_consumer<S>(
    name: &str,
    connection_type: PeerConnectionType,
    peer: Peer<S>,
) -> Result<BattlerServiceConsumer<S>>
where
    S: Send + 'static,
{
    let consumer = BattlerService::consumer(
        battler_wamprat_schema::PeerConfig {
            connection: PeerConnectionConfig::new(connection_type),
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

#[tokio::test(flavor = "multi_thread")]
async fn lists_battles() {
    let (router_handle, _) = start_router().await.unwrap();
    run_producer(router_handle.clone()).await.unwrap();
    let consumer = start_consumer(
        "player-1",
        PeerConnectionType::Remote(format!("ws://{}", router_handle.local_addr())),
        create_peer("player-1").unwrap(),
    )
    .await
    .unwrap();
    assert_matches::assert_matches!(
        consumer
            .battles(
                BattlesInput(BattlesInputArgs::default()),
                CallOptions::default(),
            )
            .await,
        Ok(battles) => {
            assert_matches::assert_matches!(battles.result().await, Ok(battles) => {
                pretty_assertions::assert_eq!(battles.0.battles, Vec::default());
            });
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn authorizes_battle_creation() {
    let (router_handle, _) = start_router().await.unwrap();
    run_producer(router_handle.clone()).await.unwrap();
    let consumer = start_consumer(
        "player-1",
        PeerConnectionType::Remote(format!("ws://{}", router_handle.local_addr())),
        create_peer("player-1").unwrap(),
    )
    .await
    .unwrap();
    assert_matches::assert_matches!(
        consumer
            .create(
                CreateInput(CreateInputArgs {
                    options_json: serde_json::to_string(&CoreBattleOptions::default()).unwrap(),
                    service_options_json: serde_json::to_string(&BattleServiceOptions::default()).unwrap(),
                }),
                CallOptions::default(),
            )
            .await,
        Ok(battles) => {
            assert_matches::assert_matches!(battles.result().await, Err(err) => {
                assert_matches::assert_matches!(err.downcast::<WampError>(), Ok(err) => {
                    assert_eq!(err.to_string(), "not allowed");
                });
            });
        }
    );
}
