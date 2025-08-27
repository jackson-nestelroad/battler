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

fn lopunny() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Lopunny",
                    "species": "Lopunny",
                    "ability": "Klutz",
                    "moves": [
                        "Thunder Wave",
                        "Fling",
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
fn klutz_suppresses_item() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = lopunny().unwrap();
    team.members[0].item = Some("Toxic Orb".to_owned());
    let mut battle = make_battle(&data, 0, team, lopunny().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn klutz_prevents_berry_from_being_eaten() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = lopunny().unwrap();
    team.members[0].item = Some("Cheri Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, lopunny().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lopunny,player-2,1|name:Thunder Wave|target:Lopunny,player-1,1",
            "status|mon:Lopunny,player-1,1|status:Paralysis",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn klutz_prevents_fling() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = lopunny().unwrap();
    team.members[0].item = Some("Cheri Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, lopunny().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lopunny,player-1,1|name:Fling|noanim",
            "fail|mon:Lopunny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn klutz_prevents_natural_gift() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = lopunny().unwrap();
    team.members[0].item = Some("Cheri Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, lopunny().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lopunny,player-1,1|name:Natural Gift|noanim",
            "fail|mon:Lopunny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
