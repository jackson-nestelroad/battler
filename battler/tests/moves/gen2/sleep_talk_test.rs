use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    Error,
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

fn snorlax() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "ability": "No Ability",
                    "moves": [
                        "Sleep Talk",
                        "Sleep Powder",
                        "Pound"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn snorlax_sleep_talk_only_moves() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "ability": "No Ability",
                    "moves": [
                        "Sleep Talk",
                        "Skull Bash",
                        "Sketch"
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
) -> Result<PublicCoreBattle, Error> {
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
fn sleep_talk_uses_random_move_while_asleep() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 253642213454, snorlax().unwrap(), snorlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snorlax,player-2,1|name:Sleep Powder|target:Snorlax,player-1,1",
            "status|mon:Snorlax,player-1,1|status:Sleep",
            "residual",
            "turn|turn:2",
            ["time"],
            "cant|mon:Snorlax,player-1,1|reason:status:Sleep",
            "move|mon:Snorlax,player-1,1|name:Sleep Talk|target:Snorlax,player-1,1",
            "move|mon:Snorlax,player-1,1|name:Pound|target:Snorlax,player-2,1",
            "split|side:1",
            "damage|mon:Snorlax,player-2,1|health:180/220",
            "damage|mon:Snorlax,player-2,1|health:82/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "curestatus|mon:Snorlax,player-1,1|status:Sleep",
            "move|mon:Snorlax,player-1,1|name:Sleep Talk|noanim",
            "fail|mon:Snorlax,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sleep_talk_fails_with_no_usable_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        snorlax_sleep_talk_only_moves().unwrap(),
        snorlax().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snorlax,player-2,1|name:Sleep Powder|target:Snorlax,player-1,1",
            "status|mon:Snorlax,player-1,1|status:Sleep",
            "residual",
            "turn|turn:2",
            ["time"],
            "cant|mon:Snorlax,player-1,1|reason:status:Sleep",
            "move|mon:Snorlax,player-1,1|name:Sleep Talk|noanim",
            "fail|mon:Snorlax,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
