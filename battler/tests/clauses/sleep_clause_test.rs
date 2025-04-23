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

fn three_snorlax() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "ability": "No Ability",
                    "moves": [
                        "Rest",
                        "Spore",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "ability": "No Ability",
                    "moves": [
                        "Rest",
                        "Spore",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "ability": "No Ability",
                    "moves": [
                        "Rest",
                        "Spore",
                        "Tackle"
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
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_rule("Sleep Clause")
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn prevents_multiple_foes_falling_asleep() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 0, three_snorlax().unwrap(), three_snorlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;move 1,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snorlax,player-1,1|name:Spore|target:Snorlax,player-2,1",
            "status|mon:Snorlax,player-2,1|status:Sleep",
            "move|mon:Snorlax,player-1,2|name:Spore|noanim",
            "activate|clause:Sleep Clause",
            "fail|mon:Snorlax,player-1,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "position:1"],
            ["switch", "player-2", "position:1"],
            "move|mon:Snorlax,player-1,1|name:Spore|noanim",
            "activate|clause:Sleep Clause",
            "fail|mon:Snorlax,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snorlax,player-1,1|name:Tackle|target:Snorlax,player-2,1",
            "split|side:1",
            ["damage", "Snorlax,player-2,1"],
            ["damage", "Snorlax,player-2,1"],
            "move|mon:Snorlax,player-2,1|name:Rest|target:Snorlax,player-2,1",
            "status|mon:Snorlax,player-2,1|status:Sleep",
            "split|side:1",
            ["heal", "Snorlax,player-2,1"],
            ["heal", "Snorlax,player-2,1"],
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 3, &expected_logs);
}

#[test]
fn ally_sleeping_does_not_activate_sleep_clause() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 0, three_snorlax().unwrap(), three_snorlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snorlax,player-1,1|name:Tackle|target:Snorlax,player-2,1",
            "split|side:1",
            ["damage", "Snorlax,player-2,1"],
            ["damage", "Snorlax,player-2,1"],
            "move|mon:Snorlax,player-2,1|name:Rest|target:Snorlax,player-2,1",
            "status|mon:Snorlax,player-2,1|status:Sleep",
            "split|side:1",
            ["heal", "Snorlax,player-2,1"],
            ["heal", "Snorlax,player-2,1"],
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Snorlax,player-1,1|name:Spore|target:Snorlax,player-2,2",
            "status|mon:Snorlax,player-2,2|status:Sleep",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
