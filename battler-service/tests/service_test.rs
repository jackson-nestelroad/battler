use std::{
    collections::BTreeSet,
    time::{
        Duration,
        SystemTime,
    },
};

use ahash::HashSet;
use anyhow::Error;
use battler::{
    BagData,
    BattleType,
    CoreBattleEngineOptions,
    CoreBattleEngineSpeedSortTieResolution,
    CoreBattleOptions,
    FieldData,
    FormatData,
    Gender,
    MonData,
    MonPersistentBattleData,
    Nature,
    PlayerData,
    PlayerOptions,
    PlayerType,
    RequestType,
    Rule,
    SideData,
    StatTable,
    TeamData,
    ValidationError,
    battle::PlayerDex,
};
use battler_service::{
    BattlePreview,
    BattleServiceOptions,
    BattleState,
    BattlerService,
    LogEntry,
    Player,
    PlayerPreview,
    PlayerState,
    Side,
    SidePreview,
    Timer,
    Timers,
};
use battler_test_utils::static_local_data_store;
use itertools::Itertools;
use tokio::{
    sync::broadcast,
    time::Instant,
};

fn mon(name: String, species: String, ability: String, moves: Vec<String>, level: u8) -> MonData {
    MonData {
        name,
        species,
        item: None,
        ability,
        moves,
        pp_boosts: Vec::default(),
        nature: Nature::Hardy,
        true_nature: None,
        gender: Gender::Female,
        evs: StatTable::default(),
        ivs: StatTable::default(),
        level,
        experience: 0,
        shiny: false,
        friendship: 255,
        ball: Some("Poké Ball".to_owned()),
        hidden_power_type: None,
        different_original_trainer: false,
        dynamax_level: 0,
        gigantamax_factor: false,
        tera_type: None,
        persistent_battle_data: MonPersistentBattleData::default(),
    }
}

fn team(level: u8) -> TeamData {
    TeamData {
        members: Vec::from_iter([
            mon(
                "Bulbasaur".to_owned(),
                "Bulbasaur".to_owned(),
                "Overgrow".to_owned(),
                Vec::from_iter(["Tackle".to_owned(), "Growl".to_owned()]),
                level,
            ),
            mon(
                "Charmander".to_owned(),
                "Charmander".to_owned(),
                "Blaze".to_owned(),
                Vec::from_iter(["Scratch".to_owned(), "Growl".to_owned()]),
                level,
            ),
            mon(
                "Squirtle".to_owned(),
                "Squirtle".to_owned(),
                "Torrent".to_owned(),
                Vec::from_iter(["Tackle".to_owned(), "Tail Whip".to_owned()]),
                level,
            ),
        ]),
        bag: BagData::default(),
    }
}

fn core_battle_options(battle_type: BattleType, team: TeamData) -> CoreBattleOptions {
    CoreBattleOptions {
        seed: Some(0),
        format: FormatData {
            battle_type: battle_type,
            rules: HashSet::from_iter([Rule::value_name("Item Clause")]),
        },
        field: FieldData::default(),
        side_1: SideData {
            name: "Side 1".to_owned(),
            players: Vec::from_iter([PlayerData {
                id: "player-1".to_owned(),
                name: "Player 1".to_owned(),
                player_type: PlayerType::Trainer,
                player_options: PlayerOptions::default(),
                team: team.clone(),
                dex: PlayerDex::default(),
            }]),
        },
        side_2: SideData {
            name: "Side 2".to_owned(),
            players: Vec::from_iter([PlayerData {
                id: "player-2".to_owned(),
                name: "Player 2".to_owned(),
                player_type: PlayerType::Trainer,
                player_options: PlayerOptions::default(),
                team: team.clone(),
                dex: PlayerDex::default(),
            }]),
        },
    }
}

async fn read_all_entries_from_log_rx_stopping_at(
    log_rx: &mut broadcast::Receiver<LogEntry>,
    stop_at: &str,
) -> Vec<String> {
    let deadline = SystemTime::now() + Duration::from_secs(10);
    let mut entries = Vec::new();
    loop {
        tokio::select! {
            entry = log_rx.recv() => {
                let entry = match entry {
                    Ok(entry) => entry,
                    _ => break,
                };
                if entry.content == stop_at {
                    break;
                }
                entries.push(entry.content);
            }
            _ = tokio::time::sleep(deadline.duration_since(SystemTime::now()).unwrap_or_default()) => {
                break;
            }
        }
    }
    entries
}

#[tokio::test(flavor = "multi_thread")]
async fn creates_battle_and_players_in_waiting_state() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, TeamData::default()),
            CoreBattleEngineOptions::default(),
            BattleServiceOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(battle.state, BattleState::Preparing);
    pretty_assertions::assert_eq!(
        battle.sides,
        Vec::from_iter([
            Side {
                name: "Side 1".to_owned(),
                players: Vec::from_iter([Player {
                    id: "player-1".to_owned(),
                    name: "Player 1".to_owned(),
                    state: PlayerState::Waiting,
                }])
            },
            Side {
                name: "Side 2".to_owned(),
                players: Vec::from_iter([Player {
                    id: "player-2".to_owned(),
                    name: "Player 2".to_owned(),
                    state: PlayerState::Waiting,
                }])
            }
        ])
    );

    assert_matches::assert_matches!(battler_service.battle(battle.uuid).await, Ok(read_battle) => {
        pretty_assertions::assert_eq!(read_battle, battle);
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn cannot_start_battle_with_empty_teams() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, TeamData::default()),
            CoreBattleEngineOptions::default(),
            BattleServiceOptions::default(),
        )
        .await
        .unwrap();
    assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<ValidationError>(), Ok(err) => {
            assert!(err.problems().contains(&"Validation failed for Player 1: Empty team is not allowed."), "{err:?}");
            assert!(err.problems().contains(&"Validation failed for Player 2: Empty team is not allowed."), "{err:?}");
        });
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn player_moves_to_ready_state_with_valid_team() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, TeamData::default()),
            CoreBattleEngineOptions::default(),
            BattleServiceOptions::default(),
        )
        .await
        .unwrap();
    assert_matches::assert_matches!(
        battler_service
            .update_team(battle.uuid, "player-1", team(5))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(battler_service.battle(battle.uuid).await, Ok(battle) => {
        assert_eq!(battle.sides[0].players[0].state, PlayerState::Ready);
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_team_fails_validation_and_resets_state() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, team(5)),
            CoreBattleEngineOptions::default(),
            BattleServiceOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(battle.sides[0].players[0].state, PlayerState::Ready);
    assert_matches::assert_matches!(battler_service.validate_player(battle.uuid, "player-1").await, Ok(validation) => {
        assert!(validation.problems.is_empty());
    });

    let mut bad_team = team(5);
    bad_team.members[0].item = Some("Leftovers".to_owned());
    bad_team.members[1].item = Some("Leftovers".to_owned());

    assert_matches::assert_matches!(
        battler_service
            .update_team(battle.uuid, "player-1", bad_team)
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(battler_service.battle(battle.uuid).await, Ok(battle) => {
        assert_eq!(battle.sides[0].players[0].state, PlayerState::Waiting);
    });

    assert_matches::assert_matches!(battler_service.validate_player(battle.uuid, "player-1").await, Ok(validation) => {
        pretty_assertions::assert_eq!(validation.problems, Vec::from_iter(["Item Leftovers appears more than 1 time."]));
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn starts_battle_and_reports_player_and_request_data() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, team(5)),
            CoreBattleEngineOptions::default(),
            BattleServiceOptions::default(),
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

    // Wait for battle to start.
    let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();
    assert_matches::assert_matches!(public_log_rx.recv().await, Ok(_));

    assert_matches::assert_matches!(
        battler_service.player_data(battle.uuid, "player-1").await,
        Ok(data) => {
            assert_eq!(data.mons.len(), 3);
        }
    );
    assert_matches::assert_matches!(
        battler_service.player_data(battle.uuid, "player-2").await,
        Ok(data) => {
            assert_eq!(data.mons.len(), 3);
        }
    );
    assert_matches::assert_matches!(
        battler_service.player_data(battle.uuid, "player-3").await,
        Err(_)
    );

    assert_matches::assert_matches!(
        battler_service.request(battle.uuid, "player-1").await,
        Ok(Some(request)) => {
            assert_eq!(request.request_type(), RequestType::Turn);
        }
    );
    assert_matches::assert_matches!(
        battler_service.request(battle.uuid, "player-2").await,
        Ok(Some(request)) => {
            assert_eq!(request.request_type(), RequestType::Turn);
        }
    );
    assert_matches::assert_matches!(
        battler_service.request(battle.uuid, "player-3").await,
        Err(_)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn plays_battle_and_finishes_and_deletes() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, team(5)),
            CoreBattleEngineOptions::default(),
            BattleServiceOptions::default(),
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

    // Wait for battle to start.
    let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();
    assert_matches::assert_matches!(public_log_rx.recv().await, Ok(_));

    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-2", "move 0")
            .await,
        Ok(())
    );

    read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:2").await;

    assert_matches::assert_matches!(battler_service.delete(battle.uuid).await, Err(err) => {
        assert_eq!(err.to_string(), "cannot delete an ongoing battle");
    });

    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-2", "forfeit")
            .await,
        Ok(())
    );

    // Wait for battle to end.
    read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "win|side:0").await;

    assert_matches::assert_matches!(battler_service.battle(battle.uuid).await, Ok(battle) => {
        assert_eq!(battle.state, BattleState::Finished);
    });

    assert_matches::assert_matches!(battler_service.delete(battle.uuid).await, Ok(()));

    pretty_assertions::assert_eq!(battler_service.battles(usize::MAX, 0).await, []);
}

#[tokio::test(flavor = "multi_thread")]
async fn returns_filtered_logs_by_side() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, team(5)),
            CoreBattleEngineOptions {
                speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                ..Default::default()
            },
            BattleServiceOptions::default(),
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

    // Wait for battle to start.
    let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();
    assert_matches::assert_matches!(public_log_rx.recv().await, Ok(_));

    // Read all logs from the battle starting; we only care to verify the first turn.
    while let Ok(_) = public_log_rx.try_recv() {}

    let mut side_1_log_rx = battler_service
        .subscribe(battle.uuid, Some(0))
        .await
        .unwrap();
    let mut side_2_log_rx = battler_service
        .subscribe(battle.uuid, Some(1))
        .await
        .unwrap();

    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-2", "move 0")
            .await,
        Ok(())
    );

    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at(&mut side_1_log_rx, "turn|turn:2").await[1..],
        [
            "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
            "damage|mon:Bulbasaur,player-2,1|health:79/100",
            "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
            "damage|mon:Bulbasaur,player-1,1|health:15/19",
            "residual",
        ],
    );
    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at(&mut side_2_log_rx, "turn|turn:2").await[1..],
        [
            "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
            "damage|mon:Bulbasaur,player-2,1|health:15/19",
            "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
            "damage|mon:Bulbasaur,player-1,1|health:79/100",
            "residual",
        ],
    );
    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:2").await[1..],
        [
            "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
            "damage|mon:Bulbasaur,player-2,1|health:79/100",
            "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
            "damage|mon:Bulbasaur,player-1,1|health:79/100",
            "residual",
        ],
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn lists_battles_in_uuid_order() {
    let battler_service = BattlerService::new(static_local_data_store());
    let mut battles = Vec::new();
    battles.push(
        battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap()
            .uuid,
    );
    battles.push(
        battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap()
            .uuid,
    );
    battles.push(
        battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap()
            .uuid,
    );

    battles.sort();

    pretty_assertions::assert_eq!(
        battler_service.battles(2, 0).await,
        Vec::from_iter([
            BattlePreview {
                uuid: battles[0],
                sides: Vec::from_iter([
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-1".to_owned(),
                            name: "Player 1".to_owned(),
                        }]),
                    },
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-2".to_owned(),
                            name: "Player 2".to_owned(),
                        }]),
                    }
                ]),
            },
            BattlePreview {
                uuid: battles[1],
                sides: Vec::from_iter([
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-1".to_owned(),
                            name: "Player 1".to_owned(),
                        }]),
                    },
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-2".to_owned(),
                            name: "Player 2".to_owned(),
                        }]),
                    }
                ]),
            }
        ])
    );

    pretty_assertions::assert_eq!(
        battler_service.battles(2, 2).await,
        Vec::from_iter([BattlePreview {
            uuid: battles[2],
            sides: Vec::from_iter([
                SidePreview {
                    players: Vec::from_iter([PlayerPreview {
                        id: "player-1".to_owned(),
                        name: "Player 1".to_owned(),
                    }]),
                },
                SidePreview {
                    players: Vec::from_iter([PlayerPreview {
                        id: "player-2".to_owned(),
                        name: "Player 2".to_owned(),
                    }]),
                }
            ]),
        }])
    );

    pretty_assertions::assert_eq!(battler_service.battles(2, 3).await, Vec::default());
}

#[tokio::test(flavor = "multi_thread")]
async fn lists_battles_for_player_in_uuid_order() {
    let battler_service = BattlerService::new(static_local_data_store());
    let mut battles = Vec::new();
    battles.push(
        battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap()
            .uuid,
    );
    battles.push(
        battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap()
            .uuid,
    );
    battles.push(
        battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap()
            .uuid,
    );

    battles.sort();

    pretty_assertions::assert_eq!(
        battler_service.battles_for_player("player-2", 2, 0).await,
        Vec::from_iter([
            BattlePreview {
                uuid: battles[0],
                sides: Vec::from_iter([
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-1".to_owned(),
                            name: "Player 1".to_owned(),
                        }]),
                    },
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-2".to_owned(),
                            name: "Player 2".to_owned(),
                        }]),
                    }
                ]),
            },
            BattlePreview {
                uuid: battles[1],
                sides: Vec::from_iter([
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-1".to_owned(),
                            name: "Player 1".to_owned(),
                        }]),
                    },
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-2".to_owned(),
                            name: "Player 2".to_owned(),
                        }]),
                    }
                ]),
            }
        ])
    );

    pretty_assertions::assert_eq!(
        battler_service.battles_for_player("player-2", 2, 2).await,
        Vec::from_iter([BattlePreview {
            uuid: battles[2],
            sides: Vec::from_iter([
                SidePreview {
                    players: Vec::from_iter([PlayerPreview {
                        id: "player-1".to_owned(),
                        name: "Player 1".to_owned(),
                    }]),
                },
                SidePreview {
                    players: Vec::from_iter([PlayerPreview {
                        id: "player-2".to_owned(),
                        name: "Player 2".to_owned(),
                    }]),
                }
            ]),
        }])
    );

    pretty_assertions::assert_eq!(
        battler_service.battles_for_player("player-2", 2, 3).await,
        Vec::default()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn returns_empty_list_for_player_with_no_battles() {
    let battler_service = BattlerService::new(static_local_data_store());
    battler_service
        .create(
            core_battle_options(BattleType::Singles, team(5)),
            CoreBattleEngineOptions::default(),
            BattleServiceOptions::default(),
        )
        .await
        .unwrap();

    pretty_assertions::assert_eq!(
        battler_service.battles_for_player("player-3", 2, 0).await,
        Vec::default()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn auto_ends_battle_on_battle_timer() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, team(5)),
            CoreBattleEngineOptions {
                speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                ..Default::default()
            },
            BattleServiceOptions {
                timers: Timers {
                    battle: Some(Timer {
                        secs: 5,
                        warnings: BTreeSet::from_iter([4, 2, 1]),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

    // Wait for the battle to automatically end.
    let deadline = Instant::now() + Duration::from_secs(10);
    assert_matches::assert_matches!(
        tokio::time::timeout_at(
            deadline,
            (async || {
                while battler_service.battle(battle.uuid).await?.state != BattleState::Finished {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Ok::<_, Error>(())
            })(),
        )
        .await,
        Ok(_)
    );

    assert_matches::assert_matches!(battler_service.full_log(battle.uuid, None).await, Ok(log) => {
        pretty_assertions::assert_eq!(
            log[(log.len() - 7)..],
            [
                "turn|turn:1",
                "-battlerservice:timer|battle|remainingsecs:5",
                "-battlerservice:timer|battle|warning|remainingsecs:4",
                "-battlerservice:timer|battle|warning|remainingsecs:2",
                "-battlerservice:timer|battle|warning|remainingsecs:1",
                "-battlerservice:timer|battle|done|remainingsecs:0",
                "tie",
            ]
        );
    });

    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Err(err) => {
            assert!(err.to_string().contains("the battle is over"), "{err:#}");
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn forfeits_on_player_timer() {
    let battler_service = BattlerService::new(static_local_data_store());
    let battle = battler_service
        .create(
            core_battle_options(BattleType::Singles, team(5)),
            CoreBattleEngineOptions {
                speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                log_time: false,
                ..Default::default()
            },
            BattleServiceOptions {
                timers: Timers {
                    player: Some(Timer {
                        secs: 5,
                        warnings: BTreeSet::from_iter([1]),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

    // Wait for battle to start.
    let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();
    assert_matches::assert_matches!(public_log_rx.recv().await, Ok(_));

    // Wait for timers to start.
    read_all_entries_from_log_rx_stopping_at(
        &mut public_log_rx,
        "-battlerservice:timer|player:player-1|remainingsecs:5",
    )
    .await;

    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Ok(())
    );

    // Wait for the battle to automatically end.
    let deadline = Instant::now() + Duration::from_secs(10);
    assert_matches::assert_matches!(
        tokio::time::timeout_at(
            deadline,
            (async || {
                while battler_service.battle(battle.uuid).await?.state != BattleState::Finished {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Ok::<_, Error>(())
            })(),
        )
        .await,
        Ok(_)
    );

    assert_matches::assert_matches!(battler_service.full_log(battle.uuid, None).await, Ok(log) => {
        pretty_assertions::assert_eq!(
            log[(log.len() - 9)..],
            [
                "turn|turn:1",
                "-battlerservice:timer|player:player-1|remainingsecs:5",
                "-battlerservice:timer|player:player-2|remainingsecs:5",
                "-battlerservice:timer|player:player-2|warning|remainingsecs:1",
                "-battlerservice:timer|player:player-2|done|remainingsecs:0",
                "continue",
                "switchout|mon:Bulbasaur,player-2,1",
                "forfeited|player:player-2",
                "win|side:0",
            ]
        );
    });

    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Err(err) => {
            assert!(err.to_string().contains("the battle is over"), "{err:#}");
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn selects_random_moves_on_action_timer() {
    let battler_service = BattlerService::new(static_local_data_store());
    let mut options = core_battle_options(BattleType::Doubles, team(5));
    options.seed = Some(444444);
    let battle = battler_service
        .create(
            options,
            CoreBattleEngineOptions {
                speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                log_time: false,
                ..Default::default()
            },
            BattleServiceOptions {
                timers: Timers {
                    action: Some(Timer {
                        secs: 5,
                        warnings: BTreeSet::default(),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

    let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();

    // Wait for turn 1.
    read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:1").await;

    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-1", "move 0,1;move 0,1")
            .await,
        Ok(())
    );

    // Wait for the battle to continue.
    read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "continue").await;

    pretty_assertions::assert_eq!(
        read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:2").await,
        [
            "move|mon:Charmander,player-1,2|name:Scratch|target:Bulbasaur,player-2,1",
            "damage|mon:Bulbasaur,player-2,1|health:79/100",
            "move|mon:Charmander,player-2,2|name:Growl|spread:Bulbasaur,player-1,1;Charmander,player-1,2",
            "unboost|mon:Bulbasaur,player-1,1|stat:atk|by:1",
            "unboost|mon:Charmander,player-1,2|stat:atk|by:1",
            "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
            "damage|mon:Bulbasaur,player-2,1|health:64/100",
            "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
            "damage|mon:Bulbasaur,player-1,1|health:79/100",
            "residual",
        ],
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn only_activates_player_timer_if_request_is_active() {
    let battler_service = BattlerService::new(static_local_data_store());
    let mut options = core_battle_options(BattleType::Singles, team(5));
    options.side_1.players[0].team.members[0].level = 100;
    let battle = battler_service
        .create(
            options,
            CoreBattleEngineOptions {
                speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                log_time: false,
                ..Default::default()
            },
            BattleServiceOptions {
                timers: Timers {
                    player: Some(Timer {
                        secs: 5,
                        warnings: BTreeSet::default(),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

    let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();

    // Wait for turn 1.
    read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:1").await;

    // In the test environment where things move extremely fast, there can be a race condition
    // between the initial timers starting and the player choices below.
    //
    // Wait for the timers to start to avoid the race condition.
    read_all_entries_from_log_rx_stopping_at(
        &mut public_log_rx,
        "-battlerservice:timer|player:player-2|remainingsecs:5",
    )
    .await;

    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-1", "move 0")
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        battler_service
            .make_choice(battle.uuid, "player-2", "move 0")
            .await,
        Ok(())
    );

    read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "continue").await;

    // Wait for battle to end.
    let log = read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "win|side:0").await;

    pretty_assertions::assert_eq!(
        log,
        [
            "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
            "damage|mon:Bulbasaur,player-2,1|health:0",
            "faint|mon:Bulbasaur,player-2,1",
            "residual",
            "-battlerservice:timer|player:player-2|remainingsecs:4",
            "-battlerservice:timer|player:player-2|done|remainingsecs:0",
            "continue",
            "forfeited|player:player-2",
        ]
    );
}
