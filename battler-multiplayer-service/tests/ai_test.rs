use std::{
    sync::Arc,
    time::Duration,
};

use ahash::{
    HashMap,
    HashSet,
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
use battler_client::{
    BattleClientEvent,
    BattlerClient,
};
use battler_multiplayer_client::BattlerMultiplayerClient;
use battler_multiplayer_service::{
    AiPlayerOptions,
    AiPlayerType,
    AiPlayers,
    BattlerMultiplayerService,
    ProposedBattleOptions,
    RandomOptions,
};
use battler_multiplayer_service_client::DirectBattlerMultiplayerServiceClient;
use battler_service::{
    BattleServiceOptions,
    BattleState,
    BattlerService,
    Timer,
    Timers,
};
use battler_service_client::battler_service_client_over_direct_service;
use battler_test_utils::static_local_data_store;

fn battler_service() -> Arc<BattlerService<'static>> {
    Arc::new(BattlerService::new(static_local_data_store()))
}

async fn battler_multiplayer_service_over_battler_service(
    battler_service: Arc<BattlerService<'static>>,
) -> Arc<BattlerMultiplayerService<'static>> {
    Arc::new(
        BattlerMultiplayerService::new(
            static_local_data_store(),
            Arc::new(battler_service_client_over_direct_service(battler_service)),
        )
        .await,
    )
}

fn team_data() -> TeamData {
    TeamData {
        members: Vec::from_iter([MonData {
            name: "Pikachu".to_owned(),
            species: "Pikachu".to_owned(),
            ability: "Static".to_owned(),
            moves: Vec::from_iter(["Headbutt".to_owned(), "Quick Attack".to_owned()]),
            level: 5,
            ..Default::default()
        }]),
        ..Default::default()
    }
}

fn battle_options_singles() -> CoreBattleOptions {
    CoreBattleOptions {
        seed: Some(0),
        format: FormatData {
            battle_type: BattleType::Singles,
            rules: hashbrown::HashSet::default(),
        },
        field: FieldData::default(),
        side_1: SideData {
            name: "Side 1".to_owned(),
            players: Vec::from_iter([PlayerData {
                id: "trainer".to_owned(),
                name: "Trainer".to_owned(),
                team: team_data(),
                ..Default::default()
            }]),
        },
        side_2: SideData {
            name: "Side 2".to_owned(),
            players: Vec::from_iter([PlayerData {
                id: "random-1".to_owned(),
                name: "Random 1".to_owned(),
                team: team_data(),
                ..Default::default()
            }]),
        },
    }
}

fn battle_options_multi() -> CoreBattleOptions {
    CoreBattleOptions {
        seed: Some(0),
        format: FormatData {
            battle_type: BattleType::Multi,
            rules: hashbrown::HashSet::default(),
        },
        field: FieldData::default(),
        side_1: SideData {
            name: "Side 1".to_owned(),
            players: Vec::from_iter([
                PlayerData {
                    id: "trainer".to_owned(),
                    name: "Trainer".to_owned(),
                    team: team_data(),
                    ..Default::default()
                },
                PlayerData {
                    id: "random-1".to_owned(),
                    name: "Random 1".to_owned(),
                    team: team_data(),
                    ..Default::default()
                },
            ]),
        },
        side_2: SideData {
            name: "Side 2".to_owned(),
            players: Vec::from_iter([
                PlayerData {
                    id: "random-2".to_owned(),
                    name: "Random 2".to_owned(),
                    team: team_data(),
                    ..Default::default()
                },
                PlayerData {
                    id: "random-3".to_owned(),
                    name: "Random 3".to_owned(),
                    team: team_data(),
                    ..Default::default()
                },
            ]),
        },
    }
}

fn battle_service_options<S>(creator: S) -> BattleServiceOptions
where
    S: Into<String>,
{
    BattleServiceOptions {
        creator: creator.into(),
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

fn proposed_battle_options<S>(
    creator: S,
    battle_options: CoreBattleOptions,
) -> ProposedBattleOptions
where
    S: Into<String>,
{
    ProposedBattleOptions {
        battle_options: battle_options,
        service_options: battle_service_options(creator),
        timeout: Duration::from_secs(30),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn hosts_singles_battle_against_random_ai() {
    battler_test_utils::collect_logs();

    let battler_service = battler_service();
    let service = battler_multiplayer_service_over_battler_service(battler_service.clone()).await;
    assert_matches::assert_matches!(
        service
            .clone()
            .create_ai_players(AiPlayers {
                players: HashMap::from_iter([(
                    "random".to_owned(),
                    AiPlayerOptions {
                        ai_type: AiPlayerType::Random(RandomOptions::default()),
                        players: HashSet::from_iter(["random-1".to_owned()]),
                    },
                )]),
            })
            .await,
        Ok(())
    );

    let client = BattlerMultiplayerClient::new(
        "trainer".to_owned(),
        Arc::new(Box::new(DirectBattlerMultiplayerServiceClient::new(
            service.clone(),
        ))),
        Arc::new(battler_service_client_over_direct_service(
            battler_service.clone(),
        )),
    );

    let battler_client = client
        .propose_and_wait_for_battle_start(proposed_battle_options(
            "trainer",
            battle_options_singles(),
        ))
        .await
        .unwrap();

    let battle = battler_service
        .battle(battler_client.battle())
        .await
        .unwrap();
    assert_eq!(battle.state, BattleState::Active);

    let mut battle_event_rx = battler_client.battle_event_rx();
    while let Ok(_) = BattlerClient::wait_for_request(&mut battle_event_rx).await {
        assert_matches::assert_matches!(battler_client.make_choice("move 0").await, Ok(()));
    }

    assert_matches::assert_matches!(
        BattlerClient::wait_for_end(&mut battle_event_rx).await,
        Ok(())
    );
    assert_eq!(*battle_event_rx.borrow(), BattleClientEvent::End);
    assert_matches::assert_matches!(battler_client.state().await.winning_side, Some(_));
}

#[tokio::test(flavor = "multi_thread")]
async fn hosts_multi_battle_against_random_ai() {
    battler_test_utils::collect_logs();

    let battler_service = battler_service();
    let service = battler_multiplayer_service_over_battler_service(battler_service.clone()).await;
    assert_matches::assert_matches!(
        service
            .clone()
            .create_ai_players(AiPlayers {
                players: HashMap::from_iter([(
                    "random".to_owned(),
                    AiPlayerOptions {
                        ai_type: AiPlayerType::Random(RandomOptions::default()),
                        players: HashSet::from_iter([
                            "random-1".to_owned(),
                            "random-2".to_owned(),
                            "random-3".to_owned()
                        ]),
                    },
                )]),
            })
            .await,
        Ok(())
    );

    let client = BattlerMultiplayerClient::new(
        "trainer".to_owned(),
        Arc::new(Box::new(DirectBattlerMultiplayerServiceClient::new(
            service.clone(),
        ))),
        Arc::new(battler_service_client_over_direct_service(
            battler_service.clone(),
        )),
    );

    let battler_client = client
        .propose_and_wait_for_battle_start(proposed_battle_options(
            "trainer",
            battle_options_multi(),
        ))
        .await
        .unwrap();

    let battle = battler_service
        .battle(battler_client.battle())
        .await
        .unwrap();
    assert_eq!(battle.state, BattleState::Active);

    let mut battle_event_rx = battler_client.battle_event_rx();
    while let Ok(_) = BattlerClient::wait_for_request(&mut battle_event_rx).await {
        assert_matches::assert_matches!(battler_client.make_choice("move 0,1").await, Ok(()));
    }

    assert_matches::assert_matches!(
        BattlerClient::wait_for_end(&mut battle_event_rx).await,
        Ok(())
    );
    assert_eq!(*battle_event_rx.borrow(), BattleClientEvent::End);
    assert_matches::assert_matches!(battler_client.state().await.winning_side, Some(_));
}

#[tokio::test(flavor = "multi_thread")]
async fn spectator_receives_updates() {
    battler_test_utils::collect_logs();

    let battler_service = battler_service();
    let service = battler_multiplayer_service_over_battler_service(battler_service.clone()).await;

    assert_matches::assert_matches!(
        service
            .clone()
            .create_ai_players(AiPlayers {
                players: HashMap::from_iter([(
                    "random".to_owned(),
                    AiPlayerOptions {
                        ai_type: AiPlayerType::Random(RandomOptions::default()),
                        players: HashSet::from_iter(["random-1".to_owned()]),
                    },
                )]),
            })
            .await,
        Ok(())
    );

    let client = BattlerMultiplayerClient::new(
        "trainer".to_owned(),
        Arc::new(Box::new(DirectBattlerMultiplayerServiceClient::new(
            service.clone(),
        ))),
        Arc::new(battler_service_client_over_direct_service(
            battler_service.clone(),
        )),
    );

    let battler_client = client
        .propose_and_wait_for_battle_start(proposed_battle_options(
            "trainer",
            battle_options_singles(),
        ))
        .await
        .unwrap();

    let battle_id = battler_client.battle();

    // Create a Spectator client
    let spectator_battler = BattlerClient::new(
        battle_id,
        "spectator_1".to_owned(),
        Arc::new(battler_service_client_over_direct_service(
            battler_service.clone(),
        )),
    )
    .await
    .unwrap();

    let mut spectator_rx = spectator_battler.battle_event_rx();

    // 1. Initial catch up update should be received
    spectator_rx.changed().await.unwrap();
    assert_matches::assert_matches!(*spectator_rx.borrow_and_update(), BattleClientEvent::Update);

    // Listen to spectator updates in the background
    let mut spectator_updates = 0;
    let mut spectator_rx_task = spectator_battler.battle_event_rx();
    let spectator_task = tokio::spawn(async move {
        loop {
            if spectator_rx_task.changed().await.is_err() {
                break;
            }
            let event = spectator_rx_task.borrow_and_update().clone();
            match event {
                BattleClientEvent::Update => spectator_updates += 1,
                BattleClientEvent::End => break,
                _ => {}
            }
        }
        spectator_updates
    });

    // Make choices to run the battle
    let mut battle_event_rx = battler_client.battle_event_rx();
    while let Ok(_) = BattlerClient::wait_for_request(&mut battle_event_rx).await {
        assert_matches::assert_matches!(battler_client.make_choice("move 0").await, Ok(()));
    }

    assert_matches::assert_matches!(
        BattlerClient::wait_for_end(&mut battle_event_rx).await,
        Ok(())
    );

    // Wait for the spectator loop to finish
    let updates_count = spectator_task.await.unwrap();

    // Spectator should have received at least one Update during active turn resolutions
    assert!(
        updates_count > 0,
        "Spectator should receive state updates during the battle"
    );
}
