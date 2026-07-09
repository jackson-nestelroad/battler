use std::{
    net::{
        IpAddr,
        Ipv4Addr,
    },
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
use battler::{
    BattleType,
    CoreBattleOptions,
    FieldData,
    FormatData,
    MonData,
    PlayerData,
    SideData,
    TeamData,
};
use battler_multiplayer_service::{
    BattlerMultiplayerServiceClient,
    ProposedBattleOptions,
    ProposedBattleResponse,
    ProposedBattleUpdate,
};
use battler_server::{
    ServerConfig,
    start_server,
};
use battler_service::{
    BattleServiceOptions,
    BattleState,
    LogEntry,
};
use battler_service_client::{
    BattlerServiceClient,
    battler_service_client_over_wamp_consumer,
};
use battler_wamp::peer::{
    Peer,
    WebSocketPeer,
    new_web_socket_peer,
};
use battler_wamprat::peer::{
    PeerConnectionConfig,
    PeerConnectionType,
};
use tokio::sync::broadcast;
use uuid::Uuid;

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    let config = battler_wamp::peer::PeerConfig {
        name: name.to_owned(),
        ..Default::default()
    };
    new_web_socket_peer(config)
}

async fn start_multiplayer_consumer<S>(
    name: &str,
    url: &str,
    peer: Peer<S>,
) -> Result<battler_multiplayer_service_schema::BattlerMultiplayerServiceConsumer<S>>
where
    S: Send + 'static,
{
    let mut connection = PeerConnectionConfig::new(PeerConnectionType::Remote(url.to_owned()));
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
    url: &str,
    peer: Peer<S>,
) -> Result<battler_service_schema::BattlerServiceConsumer<S>>
where
    S: Send + 'static,
{
    let mut connection = PeerConnectionConfig::new(PeerConnectionType::Remote(url.to_owned()));
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

fn battle_options() -> CoreBattleOptions {
    CoreBattleOptions {
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
    }
}

fn proposed_battle_options() -> ProposedBattleOptions {
    ProposedBattleOptions {
        battle_options: battle_options(),
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

async fn wait_until_battle_state(
    client: &dyn BattlerServiceClient,
    battle: Uuid,
    state: BattleState,
) -> Result<()> {
    let timeout = Duration::from_secs(5);
    let deadline = Instant::now() + timeout;
    while client.battle(battle).await?.state != state {
        if Instant::now() > deadline {
            return Err(Error::msg("deadline exceeded waiting for battle state"));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

async fn wait_for_log_line(
    log_rx: &mut broadcast::Receiver<LogEntry>,
    line_substring: &str,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() > deadline {
            return Err(Error::msg(format!(
                "timeout waiting for log line containing: {line_substring}"
            )));
        }
        tokio::select! {
            entry = log_rx.recv() => {
                if let Ok(entry) = entry {
                    if entry.content.contains(line_substring) {
                        return Ok(());
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_matchmaking_and_battle_lifecycle() {
    let mut data_dir = "../battle-data/data".to_owned();
    if !std::path::Path::new(&data_dir).is_dir() {
        data_dir = "battle-data/data".to_owned();
    }

    // 1. Start the server on port 0 (ephemeral port selection)
    let handle = start_server(ServerConfig {
        address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        port: 0,
        data_dir,
        realm_name: "battler".to_owned(),
        realm_uri: "com.battler".to_owned(),
    })
    .await
    .unwrap();

    let port = handle.router_handle.local_addr().port();
    let url = format!("ws://127.0.0.1:{port}");

    // 2. Connect Player 1 client
    let peer_1 = create_peer("player_1").unwrap();
    let multiplayer_consumer_1 = start_multiplayer_consumer("player_1", &url, peer_1)
        .await
        .unwrap();
    let multiplayer_client_1 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            multiplayer_consumer_1,
        ));

    // 3. Connect Player 2 client
    let peer_2 = create_peer("player_2").unwrap();
    let multiplayer_consumer_2 = start_multiplayer_consumer("player_2", &url, peer_2)
        .await
        .unwrap();
    let multiplayer_client_2 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            multiplayer_consumer_2,
        ));

    // 4. Subscribe Player 1 to updates
    let mut update_rx_1 = multiplayer_client_1
        .proposed_battle_updates("player_1")
        .await
        .unwrap();

    // 5. Propose a battle as Player 1
    let proposed = multiplayer_client_1
        .propose_battle(proposed_battle_options())
        .await
        .unwrap();

    // 6. Accept battle as Player 2
    let accepted = multiplayer_client_2
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

    // 7. Wait for matchmaking queue to start the battle and send the UUID
    let battle_uuid =
        read_all_updates_stopping_at_battle_id_or_timeout(&mut update_rx_1, Duration::from_secs(5))
            .await
            .expect("should have created and started the battle");

    // 8. Connect to the active battle service via WAMP and inspect it
    let battle_peer = create_peer("battle-client").unwrap();
    let battle_consumer = start_battle_consumer("player_1", &url, battle_peer)
        .await
        .unwrap();
    let battle_client = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer));

    let battle = battle_client.battle(battle_uuid).await.unwrap();
    assert_eq!(battle.state, BattleState::Active);

    // Connect Player 2 to the active battle
    let battle_peer_2 = create_peer("battle-client-2").unwrap();
    let battle_consumer_2 = start_battle_consumer("player_2", &url, battle_peer_2)
        .await
        .unwrap();
    let battle_client_2 = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer_2));

    // Subscribe to battle logs (public, player 1 side, player 2 side)
    let mut public_log_rx = battle_client.subscribe(battle_uuid, None).await.unwrap();
    let mut player_1_log_rx = battle_client.subscribe(battle_uuid, Some(0)).await.unwrap();
    let mut player_2_log_rx = battle_client_2
        .subscribe(battle_uuid, Some(1))
        .await
        .unwrap();

    // Make choices for both players
    battle_client
        .make_choice(battle_uuid, "player_1", "move 0")
        .await
        .unwrap();
    battle_client_2
        .make_choice(battle_uuid, "player_2", "move 0")
        .await
        .unwrap();

    // Wait until the battle advances to turn 2 and confirm log updates are pushed
    wait_for_log_line(&mut public_log_rx, "turn|turn:2", Duration::from_secs(5))
        .await
        .unwrap();
    wait_for_log_line(&mut player_1_log_rx, "turn|turn:2", Duration::from_secs(5))
        .await
        .unwrap();
    wait_for_log_line(&mut player_2_log_rx, "turn|turn:2", Duration::from_secs(5))
        .await
        .unwrap();

    // Forfeit the battle
    battle_client
        .make_choice(battle_uuid, "player_1", "forfeit")
        .await
        .unwrap();
    // Player 2 must also make a choice for the turn to resolve and process the forfeit
    battle_client_2
        .make_choice(battle_uuid, "player_2", "move 0")
        .await
        .unwrap();

    // Wait until the battle state is Finished
    wait_until_battle_state(battle_client.as_ref(), battle_uuid, BattleState::Finished)
        .await
        .unwrap();

    // 9. Shutdown server and clean up
    handle.shutdown().await.unwrap();
}
