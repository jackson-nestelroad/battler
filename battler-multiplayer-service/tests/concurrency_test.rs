use std::{
    sync::Arc,
    time::Duration,
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
    BattlerMultiplayerService,
    ProposedBattleOptions,
    ProposedBattleResponse,
};
use battler_service::{
    BattleServiceOptions,
    BattlerService,
};
use battler_service_client::battler_service_client_over_direct_service;
use battler_test_utils::static_local_data_store;
use tokio::time::timeout;

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

fn multi_battle_options(idx: usize) -> CoreBattleOptions {
    CoreBattleOptions {
        seed: Some(12345 + idx as u64),
        format: FormatData {
            battle_type: BattleType::Multi,
            rules: Vec::from_iter(["Standard".to_owned()]),
        },
        field: FieldData::default(),
        side_1: SideData {
            name: "Side A".to_owned(),
            players: Vec::from_iter([
                PlayerData {
                    id: format!("c_{idx}"),
                    name: format!("c_{idx}"),
                    team: TeamData::default(),
                    ..Default::default()
                },
                PlayerData {
                    id: format!("p2_{idx}"),
                    name: format!("p2_{idx}"),
                    team: TeamData::default(),
                    ..Default::default()
                },
            ]),
        },
        side_2: SideData {
            name: "Side B".to_owned(),
            players: Vec::from_iter([
                PlayerData {
                    id: format!("p3_{idx}"),
                    name: format!("p3_{idx}"),
                    team: TeamData::default(),
                    ..Default::default()
                },
                PlayerData {
                    id: format!("p4_{idx}"),
                    name: format!("p4_{idx}"),
                    team: TeamData::default(),
                    ..Default::default()
                },
            ]),
        },
    }
}

fn proposed_multi_battle_options(idx: usize) -> ProposedBattleOptions {
    ProposedBattleOptions {
        battle_options: multi_battle_options(idx),
        service_options: BattleServiceOptions {
            creator: format!("c_{idx}"),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn test_no_lock_inversion_under_high_concurrency() {
    let result = timeout(Duration::from_secs(10), async {
        let battler_service = Arc::new(BattlerService::new(static_local_data_store()));
        let mp_service = Arc::new(
            BattlerMultiplayerService::new(
                static_local_data_store(),
                Arc::new(battler_service_client_over_direct_service(
                    battler_service.clone(),
                )),
            )
            .await,
        );

        let mut tasks = Vec::new();

        // Spawn concurrent tasks doing player queries
        for i in 0..20 {
            let mp_service = mp_service.clone();
            let player = format!("player-{i}");
            tasks.push(tokio::spawn(async move {
                for _ in 0..30 {
                    let _ = mp_service.proposed_battles_for_player(&player, 10, 0).await;
                    tokio::task::yield_now().await;
                }
            }));
        }

        // Spawn task creating and fulfilling battles
        let mp_service_creator = mp_service.clone();
        let battler_service_creator = battler_service.clone();
        tasks.push(tokio::spawn(async move {
            for i in 0..5 {
                let c_id = format!("c_{i}");
                let p2_id = format!("p2_{i}");
                let p3_id = format!("p3_{i}");
                let p4_id = format!("p4_{i}");

                let proposal = mp_service_creator
                    .clone()
                    .propose_battle(proposed_multi_battle_options(i))
                    .await
                    .expect("propose_battle failed");

                let uuid = proposal.uuid;

                // Players accept
                mp_service_creator
                    .respond_to_proposed_battle(
                        uuid,
                        &p2_id,
                        &ProposedBattleResponse { accept: true },
                    )
                    .await
                    .unwrap();
                mp_service_creator
                    .respond_to_proposed_battle(
                        uuid,
                        &p3_id,
                        &ProposedBattleResponse { accept: true },
                    )
                    .await
                    .unwrap();
                mp_service_creator
                    .respond_to_proposed_battle(
                        uuid,
                        &p4_id,
                        &ProposedBattleResponse { accept: true },
                    )
                    .await
                    .unwrap();

                // Get battle UUID
                let updated = mp_service_creator.proposed_battle(uuid).await.unwrap();
                let battle_uuid = updated.battle.expect("battle should be created");

                // All 4 players submit teams to start live battle
                battler_service_creator
                    .update_team(battle_uuid, &p4_id, team_data())
                    .await
                    .unwrap();
                battler_service_creator
                    .update_team(battle_uuid, &p3_id, team_data())
                    .await
                    .unwrap();
                battler_service_creator
                    .update_team(battle_uuid, &p2_id, team_data())
                    .await
                    .unwrap();
                battler_service_creator
                    .update_team(battle_uuid, &c_id, team_data())
                    .await
                    .unwrap();

                // Housekeeping cleanup sleep
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }));

        for task in tasks {
            task.await.expect("task panicked");
        }
    })
    .await;

    assert!(
        result.is_ok(),
        "Test timed out due to deadlock/lock order inversion!"
    );
}
