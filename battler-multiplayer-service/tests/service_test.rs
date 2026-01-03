use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
        SystemTime,
    },
    usize,
};

use ahash::HashSet;
use anyhow::{
    Error,
    Result,
};
use battler::{
    BattleType,
    CoreBattleOptions,
    FieldData,
    FormatData,
    Id,
    MonData,
    PlayerData,
    Rule,
    SideData,
    TeamData,
};
use battler_multiplayer_service::{
    BattlerMultiplayerService,
    Player,
    PlayerStatus,
    ProposedBattleOptions,
    ProposedBattleRejection,
    ProposedBattleResponse,
    ProposedBattleUpdate,
    Side,
};
use battler_service::{
    BattleServiceOptions,
    BattleState,
    BattlerService,
    PlayerState,
    Timer,
    Timers,
};
use battler_service_client::battler_service_client_over_direct_service;
use battler_test_utils::static_local_data_store;
use tokio::sync::broadcast;
use uuid::Uuid;

fn battler_service() -> Arc<BattlerService<'static>> {
    Arc::new(BattlerService::new(static_local_data_store()))
}

async fn battler_multiplayer_service() -> Arc<BattlerMultiplayerService<'static>> {
    battler_multiplayer_service_over_battler_service(battler_service()).await
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
            moves: Vec::from_iter(["Headbutt".to_owned()]),
            level: 5,
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
            rules: hashbrown::HashSet::from_iter([Rule::Value {
                name: Id::from("Species Clause"),
                value: String::default(),
            }]),
        },
        field: FieldData::default(),
        side_1: SideData {
            name: "Side 1".to_owned(),
            players: Vec::from_iter([PlayerData {
                id: "player-1".to_owned(),
                name: "Player 1".to_owned(),
                team: TeamData::default(),
                ..Default::default()
            }]),
        },
        side_2: SideData {
            name: "Side 2".to_owned(),
            players: Vec::from_iter([PlayerData {
                id: "player-2".to_owned(),
                name: "Player 2".to_owned(),
                team: TeamData::default(),
                ..Default::default()
            }]),
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

fn proposed_battle_options<S>(creator: S) -> ProposedBattleOptions
where
    S: Into<String>,
{
    ProposedBattleOptions {
        battle_options: battle_options(),
        service_options: battle_service_options(creator),
        timeout: Duration::from_secs(30),
    }
}

async fn read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
    update_rx: &mut broadcast::Receiver<ProposedBattleUpdate>,
    timeout: Duration,
) -> Vec<ProposedBattleUpdate> {
    let deadline = Instant::now() + timeout;
    let mut updates = Vec::new();
    loop {
        tokio::select! {
            update = update_rx.recv() => {
                match update {
                    Ok(update) => {
                        let deleted = update.deletion_reason.is_some();
                        updates.push(update);
                        if deleted {
                            break;
                        }
                    },
                    Err(_) => break,
                }
            }
            _ = tokio::time::sleep_until(deadline.into()) => break,
        }
    }

    // Past deadline, read everything else available.
    while let Ok(update) = update_rx.try_recv() {
        let deleted = update.deletion_reason.is_some();
        updates.push(update);
        if deleted {
            break;
        }
    }

    updates
}

async fn wait_until_proposed_battle_deleted(
    service: &BattlerMultiplayerService<'_>,
    proposed_battle: Uuid,
    timeout: Duration,
) -> Result<()> {
    let deadline = SystemTime::now() + timeout;
    while SystemTime::now() < deadline {
        if let Err(_) = service.proposed_battle(proposed_battle).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(Error::msg("deadline exceeded"))
}

async fn wait_until_battle_deleted(
    service: &BattlerService<'static>,
    battle: Uuid,
    timeout: Duration,
) -> Result<()> {
    let deadline = SystemTime::now() + timeout;
    while SystemTime::now() < deadline {
        if let Err(_) = service.battle(battle).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(Error::msg("deadline exceeded"))
}

#[tokio::test(flavor = "multi_thread")]
async fn cannot_find_invalid_proposed_battle() {
    let service = battler_multiplayer_service().await;
    assert_matches::assert_matches!(service.proposed_battle(Uuid::new_v4()).await, Err(err) => {
        assert_eq!(err.to_string(), "proposed battle not found");
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn cannot_create_proposed_battle_if_not_participating() {
    let service = battler_multiplayer_service().await;
    assert_matches::assert_matches!(
        service
            .clone()
            .propose_battle(proposed_battle_options("player-3"))
            .await,
        Err(err) => {
            assert_eq!(err.to_string(), "you must participate in the battle");
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn creates_proposed_battle() {
    let service = battler_multiplayer_service().await;
    let proposed_battle = service
        .clone()
        .propose_battle(proposed_battle_options("player-1"))
        .await
        .unwrap();
    pretty_assertions::assert_eq!(
        proposed_battle.sides,
        [
            Side {
                name: "Side 1".to_owned(),
                players: Vec::from_iter([Player {
                    id: "player-1".to_owned(),
                    name: "Player 1".to_owned(),
                    status: Some(PlayerStatus::Accepted),
                }])
            },
            Side {
                name: "Side 2".to_owned(),
                players: Vec::from_iter([Player {
                    id: "player-2".to_owned(),
                    name: "Player 2".to_owned(),
                    status: None,
                }])
            }
        ]
    );
    assert_matches::assert_matches!(service.proposed_battle(proposed_battle.uuid).await, Ok(lookup) => {
        pretty_assertions::assert_eq!(lookup, proposed_battle);
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn rejection_deletes_proposed_battle() {
    let service = battler_multiplayer_service().await;
    let proposed_battle = service
        .clone()
        .propose_battle(proposed_battle_options("player-1"))
        .await
        .unwrap();
    let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

    assert_matches::assert_matches!(
        service
            .respond_to_proposed_battle(
                proposed_battle.uuid,
                "player-2",
                &ProposedBattleResponse { accept: false },
            )
            .await,
        Ok(_)
    );

    let updates = read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
        &mut update_rx,
        Duration::from_secs(5),
    )
    .await;
    assert!(updates.len() > 0, "{updates:?}");

    let update = updates.last().unwrap();
    assert_eq!(
        update.proposed_battle.sides[1].players[0].status,
        Some(PlayerStatus::Rejected)
    );
    assert_matches::assert_matches!(update.proposed_battle.battle, None);
    assert_matches::assert_matches!(&update.rejection, Some(rejection) => {
        pretty_assertions::assert_eq!(
            rejection,
            &ProposedBattleRejection {
                players: Vec::from_iter(["player-2".to_owned()]),
            }
        );
    });
    assert_matches::assert_matches!(&update.deletion_reason, Some(reason) => {
        assert_eq!(reason, "rejected");
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn creator_can_reject() {
    let service = battler_multiplayer_service().await;
    let proposed_battle = service
        .clone()
        .propose_battle(proposed_battle_options("player-1"))
        .await
        .unwrap();
    let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

    assert_matches::assert_matches!(
        service
            .respond_to_proposed_battle(
                proposed_battle.uuid,
                "player-1",
                &ProposedBattleResponse { accept: false },
            )
            .await,
        Ok(_)
    );

    let updates = read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
        &mut update_rx,
        Duration::from_secs(5),
    )
    .await;
    assert!(updates.len() > 0, "{updates:?}");

    let update = updates.last().unwrap();
    assert_eq!(
        update.proposed_battle.sides[0].players[0].status,
        Some(PlayerStatus::Rejected)
    );
    assert_matches::assert_matches!(update.proposed_battle.battle, None);
    assert_matches::assert_matches!(&update.rejection, Some(rejection) => {
        pretty_assertions::assert_eq!(
            rejection,
            &ProposedBattleRejection {
                players: Vec::from_iter(["player-1".to_owned()]),
            }
        );
    });
    assert_matches::assert_matches!(&update.deletion_reason, Some(reason) => {
        assert_eq!(reason, "rejected");
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn timeout_deletes_proposed_battle() {
    let service = battler_multiplayer_service().await;
    let mut options = proposed_battle_options("player-1");
    options.timeout = Duration::from_secs(2);
    assert_matches::assert_matches!(service.clone().propose_battle(options).await, Ok(_));

    let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

    let updates = read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
        &mut update_rx,
        Duration::from_secs(5),
    )
    .await;
    assert!(updates.len() > 0, "{updates:?}");

    let update = updates.last().unwrap();
    assert_matches::assert_matches!(update.proposed_battle.battle, None);
    assert_matches::assert_matches!(&update.rejection, None);
    assert_matches::assert_matches!(&update.deletion_reason, Some(reason) => {
        assert_eq!(reason, "deadline exceeded");
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn battle_created_when_accepted() {
    let battler_service = battler_service();
    let service = battler_multiplayer_service_over_battler_service(battler_service.clone()).await;
    let proposed_battle = service
        .clone()
        .propose_battle(proposed_battle_options("player-1"))
        .await
        .unwrap();
    let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

    assert_matches::assert_matches!(
        service
            .respond_to_proposed_battle(
                proposed_battle.uuid,
                "player-2",
                &ProposedBattleResponse { accept: true },
            )
            .await,
        Ok(_)
    );

    let update = update_rx.recv().await.unwrap();

    assert_eq!(
        update.proposed_battle.sides[1].players[0].status,
        Some(PlayerStatus::Accepted)
    );
    assert_matches::assert_matches!(&update.rejection, None);
    assert_matches::assert_matches!(&update.deletion_reason, None);

    let battle = update.proposed_battle.battle.unwrap();
    assert_matches::assert_matches!(battler_service.battle(battle).await, Ok(battle) => {
        assert_eq!(battle.state, BattleState::Preparing);
        assert_eq!(battle.sides[0].players[0].state, PlayerState::Waiting);
        assert_eq!(battle.sides[1].players[0].state, PlayerState::Waiting);
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn proposed_battle_updates_when_team_updates() {
    let battler_service = battler_service();
    let service = battler_multiplayer_service_over_battler_service(battler_service.clone()).await;
    let proposed_battle = service
        .clone()
        .propose_battle(proposed_battle_options("player-1"))
        .await
        .unwrap();
    let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

    assert_matches::assert_matches!(
        service
            .respond_to_proposed_battle(
                proposed_battle.uuid,
                "player-2",
                &ProposedBattleResponse { accept: true },
            )
            .await,
        Ok(_)
    );

    let battle = update_rx
        .recv()
        .await
        .unwrap()
        .proposed_battle
        .battle
        .unwrap();

    assert_matches::assert_matches!(
        battler_service
            .update_team(battle, "player-1", team_data())
            .await,
        Ok(())
    );

    let update = update_rx.recv().await.unwrap();
    assert_matches::assert_matches!(update.deletion_reason, None);

    // Does not pass Species Clause.
    let mut invalid_team_data = team_data();
    invalid_team_data
        .members
        .push(invalid_team_data.members[0].clone());
    assert_matches::assert_matches!(
        battler_service
            .update_team(battle, "player-2", invalid_team_data)
            .await,
        Ok(())
    );

    let update = update_rx.recv().await.unwrap();
    assert_matches::assert_matches!(update.deletion_reason, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn battle_starting_deletes_proposed_battle() {
    let battler_service = battler_service();
    let service = battler_multiplayer_service_over_battler_service(battler_service.clone()).await;
    let proposed_battle = service
        .clone()
        .propose_battle(proposed_battle_options("player-1"))
        .await
        .unwrap();
    let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

    assert_matches::assert_matches!(
        service
            .respond_to_proposed_battle(
                proposed_battle.uuid,
                "player-2",
                &ProposedBattleResponse { accept: true },
            )
            .await,
        Ok(_)
    );

    let battle = update_rx
        .recv()
        .await
        .unwrap()
        .proposed_battle
        .battle
        .unwrap();

    assert_matches::assert_matches!(
        battler_service
            .update_team(battle, "player-1", team_data())
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        battler_service
            .update_team(battle, "player-2", team_data())
            .await,
        Ok(())
    );

    let updates = read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
        &mut update_rx,
        Duration::from_secs(5),
    )
    .await;
    let update = updates.last().unwrap();

    assert_matches::assert_matches!(&update.deletion_reason, Some(reason) => {
        assert_eq!(reason, "fulfilled");
    });

    assert_matches::assert_matches!(
        wait_until_proposed_battle_deleted(&service, proposed_battle.uuid, Duration::from_secs(5))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(service.proposed_battle(proposed_battle.uuid).await, Err(err) => {
        assert_eq!(err.to_string(), "proposed battle not found");
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn rejection_deletes_underlying_battle_after_creation() {
    let battler_service = battler_service();
    let service = battler_multiplayer_service_over_battler_service(battler_service.clone()).await;
    let proposed_battle = service
        .clone()
        .propose_battle(proposed_battle_options("player-1"))
        .await
        .unwrap();
    let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

    assert_matches::assert_matches!(
        service
            .respond_to_proposed_battle(
                proposed_battle.uuid,
                "player-2",
                &ProposedBattleResponse { accept: true },
            )
            .await,
        Ok(_)
    );

    let battle = update_rx
        .recv()
        .await
        .unwrap()
        .proposed_battle
        .battle
        .unwrap();

    assert_matches::assert_matches!(
        service
            .respond_to_proposed_battle(
                proposed_battle.uuid,
                "player-2",
                &ProposedBattleResponse { accept: false },
            )
            .await,
        Ok(_)
    );

    assert_matches::assert_matches!(
        wait_until_proposed_battle_deleted(&service, proposed_battle.uuid, Duration::from_secs(5))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(
        wait_until_battle_deleted(&battler_service, battle, Duration::from_secs(5)).await,
        Ok(())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn lists_proposed_battles_for_player() {
    let service = battler_multiplayer_service().await;
    let proposed_battle_1 = service
        .clone()
        .propose_battle(proposed_battle_options("player-1"))
        .await
        .unwrap();
    let proposed_battle_2 = service
        .clone()
        .propose_battle(proposed_battle_options("player-2"))
        .await
        .unwrap();

    pretty_assertions::assert_eq!(
        service
            .proposed_battles_for_player("player-1", usize::MAX, 0)
            .await
            .into_iter()
            .map(|proposed_battle| proposed_battle.uuid)
            .collect::<HashSet<_>>(),
        HashSet::from_iter([proposed_battle_1.uuid, proposed_battle_2.uuid])
    );
    pretty_assertions::assert_eq!(
        service
            .proposed_battles_for_player("player-2", usize::MAX, 0)
            .await
            .into_iter()
            .map(|proposed_battle| proposed_battle.uuid)
            .collect::<HashSet<_>>(),
        HashSet::from_iter([proposed_battle_1.uuid, proposed_battle_2.uuid])
    );
    pretty_assertions::assert_eq!(
        service
            .proposed_battles_for_player("player-3", usize::MAX, 0)
            .await
            .into_iter()
            .map(|proposed_battle| proposed_battle.uuid)
            .collect::<HashSet<_>>(),
        HashSet::default()
    );
}
