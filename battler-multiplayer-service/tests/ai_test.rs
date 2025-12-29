use std::{
    sync::Arc,
    time::{
        Duration,
        SystemTime,
    },
};

use ahash::{
    HashMap,
    HashSet,
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
use battler_client::{
    BattleClientEvent,
    BattlerClient,
};
use battler_multiplayer_service::{
    AiPlayerOptions,
    AiPlayerType,
    AiPlayers,
    BattlerMultiplayerService,
    ProposedBattleOptions,
    ProposedBattleUpdate,
    RandomOptions,
};
use battler_service::{
    BattleServiceOptions,
    BattleState,
    BattlerService,
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
            rules: HashSet::default(),
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

async fn wait_for_battle_from_proposed_battle(
    proposed_battle: Uuid,
    proposed_battle_update_rx: &mut broadcast::Receiver<ProposedBattleUpdate>,
) -> Result<Uuid> {
    let deadline = SystemTime::now() + Duration::from_secs(10);
    loop {
        tokio::select! {
            update = proposed_battle_update_rx.recv() => {
                let update = update?;
                if update.proposed_battle.uuid == proposed_battle
                    && let Some(battle) = update.proposed_battle.battle
                {
                    return Ok(battle);
                }
            }
            _ = tokio::time::sleep(deadline.duration_since(SystemTime::now()).unwrap_or_default()) => {
                return Err(Error::msg("deadline exceeded"));
            }
        }
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

    let mut proposed_battle_update_rx = service.proposed_battle_updates("trainer").await.unwrap();

    let proposed_battle = service
        .propose_battle(proposed_battle_options("trainer", battle_options_singles()))
        .await
        .unwrap();
    let battle =
        wait_for_battle_from_proposed_battle(proposed_battle.uuid, &mut proposed_battle_update_rx)
            .await
            .unwrap();
    let battle = battler_service.battle(battle).await.unwrap();
    assert_eq!(battle.state, BattleState::Active);

    let client = BattlerClient::new(
        battle.uuid,
        "trainer".to_owned(),
        Arc::new(battler_service_client_over_direct_service(battler_service)),
    )
    .await
    .unwrap();

    let mut battle_event_rx = client.battle_event_rx();
    while let Ok(_) = BattlerClient::wait_for_request(&mut battle_event_rx).await {
        assert_matches::assert_matches!(client.make_choice("move 0").await, Ok(()));
    }

    assert_eq!(*battle_event_rx.borrow(), BattleClientEvent::End);
}
