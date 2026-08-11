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
    connection.reconnect_delay = Duration::from_secs(3600);
    connection.max_consecutive_failures = 1;
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
    connection.reconnect_delay = Duration::from_secs(3600);
    connection.max_consecutive_failures = 1;
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

fn team_1() -> TeamData {
    TeamData {
        members: Vec::from_iter([MonData {
            name: "Pikachu".to_owned(),
            species: "Pikachu".to_owned(),
            ability: "Static".to_owned(),
            moves: Vec::from_iter(["Growl".to_owned()]),
            level: 5,
            ..Default::default()
        }]),
        ..Default::default()
    }
}

fn team_2() -> TeamData {
    TeamData {
        members: Vec::from_iter([MonData {
            name: "Meowth".to_owned(),
            species: "Meowth".to_owned(),
            ability: "Pickup".to_owned(),
            moves: Vec::from_iter(["Scratch".to_owned()]),
            level: 8,
            ..Default::default()
        }]),
        ..Default::default()
    }
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
                team: TeamData::default(),
                ..Default::default()
            }]),
        },
        side_2: SideData {
            name: "Side 2".to_owned(),
            players: Vec::from_iter([PlayerData {
                id: "player_2".to_owned(),
                name: "Player 2".to_owned(),
                team: TeamData::default(),
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

    let battle_uuid = accepted.battle.expect("battle should be created");

    // Connect to the active battle service via WAMP and inspect it
    let battle_peer = create_peer("battle-client").unwrap();
    let battle_consumer = start_battle_consumer("player_1", &url, battle_peer)
        .await
        .unwrap();
    let battle_client = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer));

    // Connect Player 2 to the active battle
    let battle_peer_2 = create_peer("battle-client-2").unwrap();
    let battle_consumer_2 = start_battle_consumer("player_2", &url, battle_peer_2)
        .await
        .unwrap();
    let battle_client_2 = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer_2));

    // Submit teams so the battle can auto-start
    battle_client
        .update_team(battle_uuid, "player_1", team_1())
        .await
        .unwrap();
    battle_client_2
        .update_team(battle_uuid, "player_2", team_2())
        .await
        .unwrap();

    // 7. Wait for matchmaking queue to start the battle and send the UUID
    let started_battle_uuid =
        read_all_updates_stopping_at_battle_id_or_timeout(&mut update_rx_1, Duration::from_secs(5))
            .await
            .expect("should have created and started the battle");
    assert_eq!(started_battle_uuid, battle_uuid);

    wait_until_battle_state(battle_client.as_ref(), battle_uuid, BattleState::Active)
        .await
        .unwrap();

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

fn multi_proposed_battle_options() -> ProposedBattleOptions {
    let mut options = battle_options();
    options.format.battle_type = BattleType::Multi;
    options.side_1.players = vec![
        PlayerData {
            id: "player_1".to_owned(),
            name: "Player 1".to_owned(),
            team: TeamData::default(),
            ..Default::default()
        },
        PlayerData {
            id: "player_2".to_owned(),
            name: "Player 2".to_owned(),
            team: TeamData::default(),
            ..Default::default()
        },
    ];
    options.side_2.players = vec![
        PlayerData {
            id: "player_3".to_owned(),
            name: "Player 3".to_owned(),
            team: TeamData::default(),
            ..Default::default()
        },
        PlayerData {
            id: "player_4".to_owned(),
            name: "Player 4".to_owned(),
            team: TeamData::default(),
            ..Default::default()
        },
    ];
    ProposedBattleOptions {
        battle_options: options,
        service_options: BattleServiceOptions {
            creator: "player_1".to_owned(),
            ..Default::default()
        },
        timeout: Duration::from_secs(30),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_multi_battle_lifecycle() {
    let _ = tracing_subscriber::fmt::try_init();
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

    // Connect Player 2 client
    let peer_2 = create_peer("player_2").unwrap();
    let multiplayer_consumer_2 = start_multiplayer_consumer("player_2", &url, peer_2)
        .await
        .unwrap();
    let multiplayer_client_2 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            multiplayer_consumer_2,
        ));

    // Connect Player 3 client
    let peer_3 = create_peer("player_3").unwrap();
    let multiplayer_consumer_3 = start_multiplayer_consumer("player_3", &url, peer_3)
        .await
        .unwrap();
    let multiplayer_client_3 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            multiplayer_consumer_3,
        ));

    // Connect Player 4 client
    let peer_4 = create_peer("player_4").unwrap();
    let multiplayer_consumer_4 = start_multiplayer_consumer("player_4", &url, peer_4)
        .await
        .unwrap();
    let multiplayer_client_4 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            multiplayer_consumer_4,
        ));

    // 3. Subscribe Player 1 to updates
    let mut update_rx_1 = multiplayer_client_1
        .proposed_battle_updates("player_1")
        .await
        .unwrap();

    // 4. Propose a battle as Player 1
    let proposed = multiplayer_client_1
        .propose_battle(multi_proposed_battle_options())
        .await
        .unwrap();

    // 5. Accept battle as Player 2, 3
    for (client, player) in [
        (&multiplayer_client_2, "player_2"),
        (&multiplayer_client_3, "player_3"),
    ] {
        client
            .respond_to_proposed_battle(
                proposed.uuid,
                player,
                ProposedBattleResponse { accept: true },
            )
            .await
            .unwrap();
    }

    // 6. Accept battle as Player 4
    let accepted = multiplayer_client_4
        .respond_to_proposed_battle(
            proposed.uuid,
            "player_4",
            ProposedBattleResponse { accept: true },
        )
        .await
        .unwrap();

    let battle_uuid = accepted.battle.expect("battle should be created");

    // Connect to the active battle service via WAMP and inspect it
    let battle_peer_1 = create_peer("battle-client-1").unwrap();
    let battle_consumer_1 = start_battle_consumer("player_1", &url, battle_peer_1)
        .await
        .unwrap();
    let battle_client_1 = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer_1));

    let battle_peer_2 = create_peer("battle-client-2").unwrap();
    let battle_consumer_2 = start_battle_consumer("player_2", &url, battle_peer_2)
        .await
        .unwrap();
    let battle_client_2 = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer_2));

    let battle_peer_3 = create_peer("battle-client-3").unwrap();
    let battle_consumer_3 = start_battle_consumer("player_3", &url, battle_peer_3)
        .await
        .unwrap();
    let battle_client_3 = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer_3));

    let battle_peer_4 = create_peer("battle-client-4").unwrap();
    let battle_consumer_4 = start_battle_consumer("player_4", &url, battle_peer_4)
        .await
        .unwrap();
    let battle_client_4 = battler_service_client_over_wamp_consumer(Arc::new(battle_consumer_4));

    // Submit teams so the battle can auto-start
    battle_client_1
        .update_team(battle_uuid, "player_1", team_1())
        .await
        .unwrap();
    battle_client_2
        .update_team(battle_uuid, "player_2", team_2())
        .await
        .unwrap();
    battle_client_3
        .update_team(battle_uuid, "player_3", team_1())
        .await
        .unwrap();
    battle_client_4
        .update_team(battle_uuid, "player_4", team_2())
        .await
        .unwrap();

    // 7. Wait for matchmaking queue to start the battle and send the UUID
    let started_battle_uuid =
        read_all_updates_stopping_at_battle_id_or_timeout(&mut update_rx_1, Duration::from_secs(5))
            .await
            .expect("should have created and started the battle");
    assert_eq!(started_battle_uuid, battle_uuid);

    wait_until_battle_state(battle_client_1.as_ref(), battle_uuid, BattleState::Active)
        .await
        .unwrap();

    // 8. Wait for the proposed battle to be deleted by housekeeping
    let mut deleted = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(update) =
            tokio::time::timeout(Duration::from_millis(500), update_rx_1.recv()).await
        {
            if let Ok(update) = update {
                if update
                    .deletion_reason
                    .is_some_and(|reason| reason == "fulfilled")
                {
                    deleted = true;
                    break;
                }
            }
        }
    }
    assert!(
        deleted,
        "proposed battle should have been deleted with reason fulfilled"
    );

    // 9. Shutdown server and clean up
    handle.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_multi_battle_fulfillment_and_subsequent_proposals() {
    let _ = tracing_subscriber::fmt::try_init();
    let mut data_dir = "../battle-data/data".to_owned();
    if !std::path::Path::new(&data_dir).is_dir() {
        data_dir = "battle-data/data".to_owned();
    }

    // 1. Start server
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

    // 2. Connect Multiplayer Clients
    let peer_1 = create_peer("player_1").unwrap();
    let mp_1 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            start_multiplayer_consumer("player_1", &url, peer_1)
                .await
                .unwrap(),
        ));

    let peer_2 = create_peer("player_2").unwrap();
    let mp_2 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            start_multiplayer_consumer("player_2", &url, peer_2)
                .await
                .unwrap(),
        ));

    let peer_3 = create_peer("player_3").unwrap();
    let mp_3 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            start_multiplayer_consumer("player_3", &url, peer_3)
                .await
                .unwrap(),
        ));

    let peer_4 = create_peer("player_4").unwrap();
    let mp_4 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            start_multiplayer_consumer("player_4", &url, peer_4)
                .await
                .unwrap(),
        ));

    let peer_5 = create_peer("player_5").unwrap();
    let mp_5 =
        battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(Arc::new(
            start_multiplayer_consumer("player_5", &url, peer_5)
                .await
                .unwrap(),
        ));

    // 3. Propose Battle 1
    let proposed = mp_1
        .propose_battle(multi_proposed_battle_options())
        .await
        .unwrap();

    for (client, player) in [(&mp_2, "player_2"), (&mp_3, "player_3")] {
        client
            .respond_to_proposed_battle(
                proposed.uuid,
                player,
                ProposedBattleResponse { accept: true },
            )
            .await
            .unwrap();
    }

    let accepted = mp_4
        .respond_to_proposed_battle(
            proposed.uuid,
            "player_4",
            ProposedBattleResponse { accept: true },
        )
        .await
        .unwrap();

    let battle_uuid = accepted.battle.expect("battle should be created");

    // Connect battle consumers and update teams to start live battle
    let b_peer_1 = create_peer("b_player_1").unwrap();
    let bc_1 = battler_service_client_over_wamp_consumer(Arc::new(
        start_battle_consumer("player_1", &url, b_peer_1)
            .await
            .unwrap(),
    ));

    let b_peer_2 = create_peer("b_player_2").unwrap();
    let bc_2 = battler_service_client_over_wamp_consumer(Arc::new(
        start_battle_consumer("player_2", &url, b_peer_2)
            .await
            .unwrap(),
    ));

    let b_peer_3 = create_peer("b_player_3").unwrap();
    let bc_3 = battler_service_client_over_wamp_consumer(Arc::new(
        start_battle_consumer("player_3", &url, b_peer_3)
            .await
            .unwrap(),
    ));

    let b_peer_4 = create_peer("b_player_4").unwrap();
    let bc_4 = battler_service_client_over_wamp_consumer(Arc::new(
        start_battle_consumer("player_4", &url, b_peer_4)
            .await
            .unwrap(),
    ));

    bc_1.update_team(battle_uuid, "player_1", team_1())
        .await
        .unwrap();
    bc_2.update_team(battle_uuid, "player_2", team_2())
        .await
        .unwrap();
    bc_3.update_team(battle_uuid, "player_3", team_1())
        .await
        .unwrap();
    bc_4.update_team(battle_uuid, "player_4", team_2())
        .await
        .unwrap();

    // Wait briefly for housekeeping fulfillment cleanup
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // 4. Verify that AFTER fulfillment deletion, new proposals and player queries succeed cleanly
    //    without hanging!
    let mut new_proposal_options = proposed_battle_options();
    new_proposal_options.service_options.creator = "player_5".to_owned();
    new_proposal_options.battle_options.side_1.players[0].id = "player_5".to_owned();
    new_proposal_options.battle_options.side_1.players[0].name = "Player 5".to_owned();
    new_proposal_options.battle_options.side_2.players[0].id = "player_1".to_owned();
    new_proposal_options.battle_options.side_2.players[0].name = "Player 1".to_owned();

    let new_proposed = tokio::time::timeout(
        Duration::from_secs(5),
        mp_5.propose_battle(new_proposal_options),
    )
    .await
    .expect("propose_battle should not deadlock or timeout")
    .unwrap();

    let player_1_proposals = tokio::time::timeout(
        Duration::from_secs(5),
        mp_1.proposed_battles_for_player("player_1", 10, 0),
    )
    .await
    .expect("proposed_battles_for_player should not deadlock or timeout")
    .unwrap();

    assert!(
        !player_1_proposals.is_empty(),
        "player_1 should see the new incoming proposal"
    );
    assert!(
        player_1_proposals
            .iter()
            .any(|p| p.uuid == new_proposed.uuid),
        "player_1 proposals should contain the new proposal"
    );

    handle.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_stress_concurrent_battle_fulfillment_and_queries() {
    let _ = tracing_subscriber::fmt::try_init();
    let mut data_dir = "../battle-data/data".to_owned();
    if !std::path::Path::new(&data_dir).is_dir() {
        data_dir = "battle-data/data".to_owned();
    }

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

    // Run 10 parallel workers hammering proposed battle creation, queries, and team updates
    // concurrently.
    let mut tasks = Vec::new();
    for i in 0..10 {
        let url = url.clone();
        tasks.push(tokio::spawn(async move {
            let p_id = format!("stress_p_{i}");
            let peer = create_peer(&p_id).unwrap();
            let mp = battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(
                Arc::new(start_multiplayer_consumer(&p_id, &url, peer).await.unwrap()),
            );

            let opp_id = format!("stress_p_{}", (i + 1) % 10);
            let mut options = proposed_battle_options();
            options.service_options.creator = p_id.clone();
            options.battle_options.side_1.players[0].id = p_id.clone();
            options.battle_options.side_2.players[0].id = opp_id.clone();

            for j in 0..5 {
                log::info!("Task {} iter {} starting propose_battle", p_id, j);
                if let Ok(_proposed) = mp.propose_battle(options.clone()).await {
                    log::info!("Task {} iter {} proposed_battle success", p_id, j);
                    let _ = mp.proposed_battles_for_player(&p_id, 10, 0).await;
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    let _ = mp.proposed_battles_for_player(&opp_id, 10, 0).await;
                    log::info!("Task {} iter {} queries done", p_id, j);
                } else {
                    log::info!("Task {} iter {} proposed_battle failed", p_id, j);
                }
            }
        }));
    }

    for (i, task) in tasks.into_iter().enumerate() {
        let _ = tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .unwrap_or_else(|_| panic!("stress task {i} deadlocked or timed out"));
    }

    log::info!("Shutting down server...");
    handle.shutdown().await.unwrap();
    log::info!("Test completely finished!");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_stress_concurrent_live_battles_and_timers() -> Result<()> {
    let mut data_dir = "../battle-data/data".to_owned();
    if !std::path::Path::new(&data_dir).is_dir() {
        data_dir = "battle-data/data".to_owned();
    }

    let handle = start_server(ServerConfig {
        address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        port: 0,
        data_dir,
        realm_name: "battler".to_owned(),
        realm_uri: "com.battler".to_owned(),
    })
    .await?;

    let port = handle.router_handle.local_addr().port();
    let url = format!("ws://127.0.0.1:{port}");

    let mut tasks = Vec::new();
    for i in 0..10 {
        let url = url.clone();
        tasks.push(tokio::spawn(async move {
            let p_id = format!("live_stress_p_{i}");
            let peer1 = create_peer(&format!("{p_id}-svc")).unwrap();
            let peer2 = create_peer(&format!("{p_id}-mp")).unwrap();

            let svc_consumer = start_battle_consumer(&p_id, &url, peer1).await.unwrap();
            let mp_consumer = start_multiplayer_consumer(&p_id, &url, peer2)
                .await
                .unwrap();

            let svc = battler_service_client_over_wamp_consumer(Arc::new(svc_consumer));
            let mp = battler_multiplayer_service_client::WampBattlerMultiplayerServiceClient::new(
                Arc::new(mp_consumer),
            );

            let mut options = proposed_battle_options();
            options.service_options.creator = p_id.clone();
            options.battle_options.side_1.players[0].id = p_id.clone();
            options.battle_options.side_2.players[0].id = format!("live_stress_opp_{i}");

            for _ in 0..2 {
                if let Ok(proposed) = mp.propose_battle(options.clone()).await {
                    let _ = svc.battles_for_player(&p_id, 10, 0).await;
                    tokio::time::sleep(Duration::from_millis(15)).await;

                    if let Some(battle_uuid) = proposed.battle {
                        let _ = svc.update_team(battle_uuid, &p_id, team_1()).await;
                        let _ = svc.start(battle_uuid).await;

                        for _ in 0..2 {
                            let _ = svc.battles_for_player(&p_id, 10, 0).await;
                            let _ = svc.make_choice(battle_uuid, &p_id, "move 1").await;
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    }
                }
            }
        }));
    }

    for task in tasks {
        tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .expect("live battle stress task timed out")
            .unwrap();
    }

    handle.shutdown().await.unwrap();
    Ok(())
}
