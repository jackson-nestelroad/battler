use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn typhlosion_low_friendship() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Typhlosion",
                    "species": "Typhlosion",
                    "ability": "No Ability",
                    "moves": [
                        "Return",
                        "Frustration"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "friendship": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn typhlosion_max_friendship() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Typhlosion",
                    "species": "Typhlosion",
                    "ability": "No Ability",
                    "moves": [
                        "Return",
                        "Frustration"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "friendship": 255
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn return_power_depends_on_friendship() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        typhlosion_low_friendship().unwrap(),
        typhlosion_max_friendship().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Typhlosion,player-1,1|name:Return|target:Typhlosion,player-2,1",
            "split|side:1",
            "damage|mon:Typhlosion,player-2,1|health:119/138",
            "damage|mon:Typhlosion,player-2,1|health:87/100",
            "move|mon:Typhlosion,player-2,1|name:Return|target:Typhlosion,player-1,1",
            "split|side:0",
            "damage|mon:Typhlosion,player-1,1|health:93/138",
            "damage|mon:Typhlosion,player-1,1|health:68/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn frustration_power_depends_on_friendship() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        typhlosion_low_friendship().unwrap(),
        typhlosion_max_friendship().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Typhlosion,player-1,1|name:Frustration|target:Typhlosion,player-2,1",
            "split|side:1",
            "damage|mon:Typhlosion,player-2,1|health:108/138",
            "damage|mon:Typhlosion,player-2,1|health:79/100",
            "move|mon:Typhlosion,player-2,1|name:Frustration|target:Typhlosion,player-1,1",
            "split|side:0",
            "damage|mon:Typhlosion,player-1,1|health:137/138",
            "damage|mon:Typhlosion,player-1,1|health:99/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
