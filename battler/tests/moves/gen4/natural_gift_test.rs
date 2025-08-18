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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn munchlax() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Munchlax",
                    "species": "Munchlax",
                    "ability": "No Ability",
                    "moves": [
                        "Natural Gift"
                    ],
                    "nature": "Hardy",
                    "level": 50
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
fn natural_gift_fails_with_no_item() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, munchlax().unwrap(), munchlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Munchlax,player-1,1|name:Natural Gift|noanim",
            "fail|mon:Munchlax,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn natural_gift_fails_with_non_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = munchlax().unwrap();
    team.members[0].item = Some("Macho Brace".to_owned());
    let mut battle = make_battle(&data, 0, team, munchlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Munchlax,player-1,1|name:Natural Gift|noanim",
            "fail|mon:Munchlax,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn natural_gift_uses_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = munchlax().unwrap();
    team.members[0].item = Some("Cheri Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, munchlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Munchlax,player-1,1|name:Natural Gift|target:Munchlax,player-2,1",
            "split|side:1",
            "damage|mon:Munchlax,player-2,1|health:126/195",
            "damage|mon:Munchlax,player-2,1|health:65/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Munchlax,player-1,1|name:Natural Gift|noanim",
            "fail|mon:Munchlax,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn natural_gift_changes_type_based_on_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = munchlax().unwrap();
    team.members[0].item = Some("Leppa Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, munchlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Munchlax,player-1,1|name:Natural Gift|target:Munchlax,player-2,1",
            "supereffective|mon:Munchlax,player-2,1",
            "split|side:1",
            "damage|mon:Munchlax,player-2,1|health:57/195",
            "damage|mon:Munchlax,player-2,1|health:30/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Munchlax,player-1,1|name:Natural Gift|noanim",
            "fail|mon:Munchlax,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
