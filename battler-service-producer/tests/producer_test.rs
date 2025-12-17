use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
    usize,
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
use battler_service::{
    BattleServiceOptions,
    BattleState,
    LogEntry,
    Timer,
    Timers,
};
use battler_service_client::{
    BattlerServiceClient,
    battler_service_client_over_wamp_consumer,
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
        hash::HashSet,
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
    sync::{
        broadcast,
        oneshot,
    },
    task::JoinHandle,
};
use uuid::Uuid;

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

#[async_trait]
impl BattleAuthorizer for Authorizer {
    async fn authorize_new_battle(
        &self,
        peer_info: &PeerInfo,
        _: &CoreBattleOptions,
    ) -> Result<()> {
        match peer_info.connection_type {
            ConnectionType::Direct => Ok(()),
            _ => Err(Error::msg("not allowed")),
        }
    }
}

async fn run_producer(router: RouterHandle) -> Result<JoinHandle<Result<()>>> {
    let data = static_local_data_store();
    let peer = new_web_socket_peer(battler_wamp::peer::PeerConfig::default())?;
    let config = battler_wamprat_schema::PeerConfig {
        connection: PeerConnectionConfig::new(PeerConnectionType::Direct(router)),
        auth_methods: Vec::default(),
    };
    let (started_tx, started_rx) = oneshot::channel();
    let handle = tokio::spawn(run_battler_service_producer(
        data,
        CoreBattleEngineOptions {
            log_time: false,
            ..Default::default()
        },
        config,
        peer,
        Modules {
            authorizer: Box::new(Authorizer),
            stop_rx: None,
            started_tx: Some(started_tx),
        },
    ));
    started_rx.await?;
    Ok(handle)
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

fn new_client<S>(consumer: BattlerServiceConsumer<S>) -> Box<dyn BattlerServiceClient>
where
    S: Send + 'static,
{
    battler_service_client_over_wamp_consumer(Arc::new(consumer))
}

fn battle_options() -> CoreBattleOptions {
    CoreBattleOptions {
        seed: Some(0),
        format: FormatData {
            battle_type: BattleType::Singles,
            rules: HashSet::default(),
        },
        field: FieldData::default(),
        side_1: SideData {
            name: "Side 1".to_owned(),
            players: Vec::from_iter([PlayerData {
                id: "player-1".to_owned(),
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
                id: "player-2".to_owned(),
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
    }
}

fn battle_service_options() -> BattleServiceOptions {
    BattleServiceOptions {
        timers: Timers {
            battle: Some(Timer {
                secs: 60,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

async fn wait_until_battle_state(
    client: &dyn BattlerServiceClient,
    battle: Uuid,
    state: BattleState,
) -> Result<()> {
    let timeout = Duration::from_secs(5);
    let deadline = Instant::now() + timeout;
    while client.battle(battle).await?.state != state {
        if Instant::now() > deadline {
            return Err(Error::msg("deadline exceeded"));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}

async fn wait_until_battle_turn(
    client: &dyn BattlerServiceClient,
    battle: Uuid,
    turn: u64,
) -> Result<()> {
    let timeout = Duration::from_secs(5);
    let deadline = Instant::now() + timeout;
    while client.battle(battle).await?.status.turn < turn {
        if Instant::now() > deadline {
            return Err(Error::msg("deadline exceeded"));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}

async fn read_all_entries_from_log_rx_stopping_at_line_or_timeout(
    log_rx: &mut broadcast::Receiver<LogEntry>,
    stop_at: &str,
    timeout: Duration,
) -> Vec<String> {
    let deadline = Instant::now() + timeout;
    let mut entries = Vec::new();
    loop {
        tokio::select! {
            entry = log_rx.recv() => {
                if let Ok(entry) = entry && entry.content != stop_at {
                    entries.push(entry.content);
                    continue;
                }
                break;
            }
            _ = tokio::time::sleep_until(deadline.into()) => break,
        }
    }

    // Past deadline, read everything else available.
    while let Ok(entry) = log_rx.try_recv()
        && entry.content != stop_at
    {
        entries.push(entry.content);
    }

    entries
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

#[tokio::test(flavor = "multi_thread")]
async fn creates_battle() {
    let (router_handle, _) = start_router().await.unwrap();
    run_producer(router_handle.clone()).await.unwrap();
    let player_1 = new_client(
        start_consumer(
            "player-1",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-1").unwrap(),
        )
        .await
        .unwrap(),
    );
    let player_2 = new_client(
        start_consumer(
            "player-2",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-2").unwrap(),
        )
        .await
        .unwrap(),
    );
    let battle = player_1
        .create(battle_options(), battle_service_options())
        .await
        .unwrap();

    assert_matches::assert_matches!(player_2.battle(battle.uuid).await, Ok(read) => {
        pretty_assertions::assert_eq!(read, battle);
    });

    assert_matches::assert_matches!(player_2.battles(usize::MAX, 0).await, Ok(battles) => {
        pretty_assertions::assert_eq!(
            battles.into_iter().map(|battle| battle.uuid).collect::<Vec<_>>(),
            [battle.uuid],
        );
    });

    assert_matches::assert_matches!(
        player_2.battles_for_player("player-2", usize::MAX, 0).await,
        Ok(battles) => {
            pretty_assertions::assert_eq!(
                battles.into_iter().map(|battle| battle.uuid).collect::<Vec<_>>(),
                [battle.uuid],
            );
        }
    );

    assert_matches::assert_matches!(
        player_2.battles_for_player("player-3", usize::MAX, 0).await,
        Ok(battles) => {
            pretty_assertions::assert_eq!(battles, []);
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_can_start_and_delete_battle() {
    let (router_handle, _) = start_router().await.unwrap();
    run_producer(router_handle.clone()).await.unwrap();
    let player_1 = new_client(
        start_consumer(
            "player-1",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-1").unwrap(),
        )
        .await
        .unwrap(),
    );
    let player_2 = new_client(
        start_consumer(
            "player-2",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-2").unwrap(),
        )
        .await
        .unwrap(),
    );
    let battle = player_1
        .create(battle_options(), battle_service_options())
        .await
        .unwrap();

    assert_matches::assert_matches!(player_2.delete(battle.uuid).await, Err(err) => {
        assert_eq!(err.to_string(), "player-2 does not own the battle");
    });
    assert_matches::assert_matches!(player_1.delete(battle.uuid).await, Err(err) => {
        assert_eq!(err.to_string(), "cannot delete an ongoing battle");
    });

    assert_matches::assert_matches!(player_2.start(battle.uuid).await, Err(err) => {
        assert_eq!(err.to_string(), "player-2 does not own the battle");
    });
    assert_matches::assert_matches!(player_1.start(battle.uuid).await, Ok(()));

    wait_until_battle_state(player_1.as_ref(), battle.uuid, BattleState::Active)
        .await
        .unwrap();

    assert_matches::assert_matches!(
        player_1
            .make_choice(battle.uuid, "player-1", "forfeit")
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        player_2
            .make_choice(battle.uuid, "player-2", "move 0")
            .await,
        Ok(())
    );

    wait_until_battle_state(player_1.as_ref(), battle.uuid, BattleState::Finished)
        .await
        .unwrap();

    assert_matches::assert_matches!(player_1.delete(battle.uuid).await, Ok(()));

    assert_matches::assert_matches!(player_2.battles(usize::MAX, 0).await, Ok(battles) => {
        pretty_assertions::assert_eq!(battles, []);
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn player_can_update_team() {
    let (router_handle, _) = start_router().await.unwrap();
    run_producer(router_handle.clone()).await.unwrap();
    let player_1 = new_client(
        start_consumer(
            "player-1",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-1").unwrap(),
        )
        .await
        .unwrap(),
    );
    let battle = player_1
        .create(battle_options(), battle_service_options())
        .await
        .unwrap();

    assert_matches::assert_matches!(
        player_1
            .update_team(
                battle.uuid,
                "player-1",
                battle_options().side_2.players[0].team.clone()
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        player_1
            .update_team(
                battle.uuid,
                "player-2",
                battle_options().side_1.players[0].team.clone()
            )
            .await,
        Err(err) => {
            assert_eq!(err.to_string(), "player-1 cannot act as player-2");
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn player_can_participate_in_battle() {
    let (router_handle, _) = start_router().await.unwrap();
    run_producer(router_handle.clone()).await.unwrap();
    let player_1 = new_client(
        start_consumer(
            "player-1",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-1").unwrap(),
        )
        .await
        .unwrap(),
    );
    let battle = player_1
        .create(battle_options(), battle_service_options())
        .await
        .unwrap();

    assert_matches::assert_matches!(player_1.start(battle.uuid).await, Ok(()));

    wait_until_battle_state(player_1.as_ref(), battle.uuid, BattleState::Active)
        .await
        .unwrap();

    assert_matches::assert_matches!(player_1.player_data(battle.uuid, "player-1").await, Ok(_));
    assert_matches::assert_matches!(player_1.player_data(battle.uuid, "player-2").await, Err(err) => {
        assert_eq!(err.to_string(), "player-1 cannot act as player-2");
    });

    assert_matches::assert_matches!(player_1.request(battle.uuid, "player-1").await, Ok(_));
    assert_matches::assert_matches!(player_1.request(battle.uuid, "player-2").await, Err(err) => {
        assert_eq!(err.to_string(), "player-1 cannot act as player-2");
    });

    assert_matches::assert_matches!(
        player_1
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Ok(_)
    );
    assert_matches::assert_matches!(player_1.make_choice(battle.uuid, "player-2", "move 0").await, Err(err) => {
        assert_eq!(err.to_string(), "player-1 cannot act as player-2");
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn publishes_battle_logs() {
    let (router_handle, _) = start_router().await.unwrap();
    run_producer(router_handle.clone()).await.unwrap();
    let player_1 = new_client(
        start_consumer(
            "player-1",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-1").unwrap(),
        )
        .await
        .unwrap(),
    );
    let player_2 = new_client(
        start_consumer(
            "player-2",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-2").unwrap(),
        )
        .await
        .unwrap(),
    );
    let player_3 = new_client(
        start_consumer(
            "player-3",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-3").unwrap(),
        )
        .await
        .unwrap(),
    );
    let battle = player_1
        .create(battle_options(), battle_service_options())
        .await
        .unwrap();

    let mut player_1_side_1_rx = player_1.subscribe(battle.uuid, Some(0)).await.unwrap();
    let mut player_1_side_2_rx = player_1.subscribe(battle.uuid, Some(1)).await.unwrap();
    let mut player_2_side_1_rx = player_2.subscribe(battle.uuid, Some(0)).await.unwrap();
    let mut player_2_side_2_rx = player_2.subscribe(battle.uuid, Some(1)).await.unwrap();
    let mut player_3_side_1_rx = player_3.subscribe(battle.uuid, Some(0)).await.unwrap();
    let mut public_rx = player_3.subscribe(battle.uuid, None).await.unwrap();

    assert_matches::assert_matches!(player_1.start(battle.uuid).await, Ok(()));

    wait_until_battle_state(player_1.as_ref(), battle.uuid, BattleState::Active)
        .await
        .unwrap();

    assert_matches::assert_matches!(
        player_1
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        player_2
            .make_choice(battle.uuid, "player-2", "move 0")
            .await,
        Ok(())
    );

    wait_until_battle_turn(player_1.as_ref(), battle.uuid, 2)
        .await
        .unwrap();

    // Players not on the corresponding side will never receive private logs.
    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at_line_or_timeout(
            &mut player_1_side_2_rx,
            "turn|turn:2",
            Duration::from_secs(3),
        )
        .await,
        Vec::<String>::default()
    );
    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at_line_or_timeout(
            &mut player_2_side_1_rx,
            "turn|turn:2",
            Duration::from_secs(3),
        )
        .await,
        Vec::<String>::default()
    );
    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at_line_or_timeout(
            &mut player_3_side_1_rx,
            "turn|turn:2",
            Duration::from_secs(3),
        )
        .await,
        Vec::<String>::default()
    );

    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at_line_or_timeout(
            &mut player_1_side_1_rx,
            "turn|turn:2",
            Duration::from_secs(3),
        )
        .await,
        [
            "info|battletype:Singles",
            "info|environment:Normal|time:Day",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "continue",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "battlestart",
            "switch|player:player-1|position:1|name:Pikachu|health:18/18|species:Pikachu|level:5|gender:U",
            "switch|player:player-2|position:1|name:Meowth|health:100/100|species:Meowth|level:5|gender:U",
            "turn|turn:1",
            "-battlerservice:timer|battle|remainingsecs:60",
            "continue",
            "move|mon:Meowth,player-2,1|name:Scratch|target:Pikachu,player-1,1",
            "damage|mon:Pikachu,player-1,1|health:12/18",
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Meowth,player-2,1",
            "damage|mon:Meowth,player-2,1|health:74/100",
            "residual",
            "-battlerservice:timer|battle|remainingsecs:59",
        ]
    );

    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at_line_or_timeout(
            &mut player_2_side_2_rx,
            "turn|turn:2",
            Duration::from_secs(3),
        )
        .await,
        [
            "info|battletype:Singles",
            "info|environment:Normal|time:Day",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "continue",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "battlestart",
            "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:5|gender:U",
            "switch|player:player-2|position:1|name:Meowth|health:19/19|species:Meowth|level:5|gender:U",
            "turn|turn:1",
            "-battlerservice:timer|battle|remainingsecs:60",
            "continue",
            "move|mon:Meowth,player-2,1|name:Scratch|target:Pikachu,player-1,1",
            "damage|mon:Pikachu,player-1,1|health:67/100",
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Meowth,player-2,1",
            "damage|mon:Meowth,player-2,1|health:14/19",
            "residual",
            "-battlerservice:timer|battle|remainingsecs:59",
        ]
    );

    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at_line_or_timeout(
            &mut public_rx,
            "turn|turn:2",
            Duration::from_secs(5),
        )
        .await,
        [
            "info|battletype:Singles",
            "info|environment:Normal|time:Day",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "continue",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "battlestart",
            "switch|player:player-1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:5|gender:U",
            "switch|player:player-2|position:1|name:Meowth|health:100/100|species:Meowth|level:5|gender:U",
            "turn|turn:1",
            "-battlerservice:timer|battle|remainingsecs:60",
            "continue",
            "move|mon:Meowth,player-2,1|name:Scratch|target:Pikachu,player-1,1",
            "damage|mon:Pikachu,player-1,1|health:67/100",
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Meowth,player-2,1",
            "damage|mon:Meowth,player-2,1|health:74/100",
            "residual",
            "-battlerservice:timer|battle|remainingsecs:59",
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn player_reads_full_log() {
    let (router_handle, _) = start_router().await.unwrap();
    run_producer(router_handle.clone()).await.unwrap();
    let player_1 = new_client(
        start_consumer(
            "player-1",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-1").unwrap(),
        )
        .await
        .unwrap(),
    );
    let player_2 = new_client(
        start_consumer(
            "player-2",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-2").unwrap(),
        )
        .await
        .unwrap(),
    );
    let player_3 = new_client(
        start_consumer(
            "player-3",
            PeerConnectionType::Direct(router_handle.clone()),
            create_peer("player-3").unwrap(),
        )
        .await
        .unwrap(),
    );
    let battle = player_1
        .create(battle_options(), battle_service_options())
        .await
        .unwrap();

    assert_matches::assert_matches!(player_1.start(battle.uuid).await, Ok(()));

    wait_until_battle_state(player_1.as_ref(), battle.uuid, BattleState::Active)
        .await
        .unwrap();

    assert_matches::assert_matches!(
        player_1.full_log(battle.uuid, None).await,
        Ok(log) => {
            assert!(!log.is_empty());
        }
    );
    assert_matches::assert_matches!(
        player_1.full_log(battle.uuid, Some(0)).await,
        Ok(log) => {
            assert!(!log.is_empty());
        }
    );
    assert_matches::assert_matches!(
        player_1.full_log(battle.uuid, Some(1)).await,
        Err(err) => {
            assert_eq!(err.to_string(), "player-1 is not on given side");
        }
    );

    assert_matches::assert_matches!(
        player_2.full_log(battle.uuid, None).await,
        Ok(log) => {
            assert!(!log.is_empty());
        }
    );
    assert_matches::assert_matches!(
        player_2.full_log(battle.uuid, Some(0)).await,
        Err(err) => {
            assert_eq!(err.to_string(), "player-2 is not on given side");
        }
    );
    assert_matches::assert_matches!(
        player_2.full_log(battle.uuid, Some(1)).await,
        Ok(log) => {
            assert!(!log.is_empty());
        }
    );

    assert_matches::assert_matches!(
        player_3.full_log(battle.uuid, None).await,
        Ok(log) => {
            assert!(!log.is_empty());
        }
    );
    assert_matches::assert_matches!(
        player_3.full_log(battle.uuid, Some(0)).await,
        Err(err) => {
            assert_eq!(err.to_string(), "player-3 is not on given side");
        }
    );
    assert_matches::assert_matches!(
        player_3.full_log(battle.uuid, Some(1)).await,
        Err(err) => {
            assert_eq!(err.to_string(), "player-3 is not on given side");
        }
    );
}
