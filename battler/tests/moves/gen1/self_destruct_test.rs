use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_turn_logs_eq,
    LogMatch,
    TestBattleBuilder,
};

fn test_team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Self-Destruct",
                        "Memento"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn foe_team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Tackle",
                        "Sand Attack"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_seed(0)
        .with_battle_type(BattleType::Singles)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("test-player", "Test Player")
        .add_player_to_side_2("foe", "Foe")
        .with_team("test-player", test_team()?)
        .with_team("foe", foe_team()?)
        .build(data)
}

#[test]
fn self_destruct_loses() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("test-player", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,test-player,1|name:Self-Destruct",
            "split|side:1",
            "damage|mon:Venusaur,foe,1|health:0",
            "damage|mon:Venusaur,foe,1|health:0",
            "faint|mon:Venusaur,test-player,1",
            "faint|mon:Venusaur,foe,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn user_self_destructs_even_if_missed() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("test-player", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("test-player", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("test-player", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("test-player", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,test-player,1|name:Self-Destruct|noanim",
            "miss|mon:Venusaur,foe,1",
            "faint|mon:Venusaur,test-player,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 4, &expected_logs);
}

#[test]
fn user_self_destructs_only_if_move_hits() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("test-player", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,test-player,1|name:Memento|target:Venusaur,foe,1",
            "unboost|mon:Venusaur,foe,1|stat:atk|by:2",
            "unboost|mon:Venusaur,foe,1|stat:spa|by:2",
            "faint|mon:Venusaur,test-player,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn user_does_not_self_destruct_if_move_misses() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("test-player", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("test-player", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("test-player", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("test-player", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("foe", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,test-player,1|name:Memento|noanim",
            "miss|mon:Venusaur,foe,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 4, &expected_logs);
}
