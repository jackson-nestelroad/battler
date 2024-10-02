use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn pikachu_team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Thunder",
                        "Sand Attack",
                        "Double Team",
                        "Fury Attack",
                        "Triple Kick",
                        "Chip Away"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn doubles_pikachu_team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Sand Attack",
                        "Double Team",
                        "Icy Wind"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Sand Attack",
                        "Double Team",
                        "Icy Wind"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_singles_battle(
    data: &dyn DataStore,
    seed: u64,
    team: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team.clone())
        .with_team("player-2", team)
        .build(data)
}

fn make_doubles_battle(
    data: &dyn DataStore,
    seed: u64,
    team: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team.clone())
        .with_team("player-2", team)
        .build(data)
}

#[test]
fn accuracy_check_applies_normally() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data, 143256777503747, pikachu_team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1",
            "resisted|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:60/95",
            "damage|mon:Pikachu,player-2,1|health:64/100",
            "move|mon:Pikachu,player-2,1|name:Thunder|noanim",
            "miss|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1",
            "resisted|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:27/95",
            "damage|mon:Pikachu,player-2,1|health:29/100",
            "move|mon:Pikachu,player-2,1|name:Thunder|target:Pikachu,player-1,1",
            "resisted|mon:Pikachu,player-1,1",
            "crit|mon:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:40/95",
            "damage|mon:Pikachu,player-1,1|health:43/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn accuracy_check_impacted_by_lowered_accuracy() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data, 716958313281881, pikachu_team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "move|mon:Pikachu,player-2,1|name:Sand Attack|noanim",
            "miss|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "move|mon:Pikachu,player-2,1|name:Sand Attack|target:Pikachu,player-1,1",
            "unboost|mon:Pikachu,player-1,1|stat:acc|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "move|mon:Pikachu,player-2,1|name:Sand Attack|target:Pikachu,player-1,1",
            "unboost|mon:Pikachu,player-1,1|stat:acc|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Thunder|noanim",
            "miss|mon:Pikachu,player-2,1",
            "move|mon:Pikachu,player-2,1|name:Thunder|noanim",
            "miss|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1",
            "resisted|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:61/95",
            "damage|mon:Pikachu,player-2,1|health:65/100",
            "move|mon:Pikachu,player-2,1|name:Thunder|noanim",
            "miss|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn accuracy_check_impacted_by_raised_evasion() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data, 0, pikachu_team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Thunder|target:Pikachu,player-2,1",
            "resisted|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:59/95",
            "damage|mon:Pikachu,player-2,1|health:63/100",
            "move|mon:Pikachu,player-2,1|name:Thunder|noanim",
            "miss|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Thunder|noanim",
            "miss|mon:Pikachu,player-2,1",
            "move|mon:Pikachu,player-2,1|name:Thunder|noanim",
            "miss|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn accuracy_check_for_each_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_doubles_battle(&data, 65564654, doubles_pikachu_team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_eq!(
        battle.set_player_choice("player-2", "move 0,1;move 1"),
        Ok(())
    );

    assert_eq!(battle.set_player_choice("player-1", "move 2;pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Sand Attack|target:Pikachu,player-1,1",
            "unboost|mon:Pikachu,player-1,1|stat:acc|by:1",
            "move|mon:Pikachu,player-2,2|name:Double Team|target:Pikachu,player-2,2",
            "boost|mon:Pikachu,player-2,2|stat:eva|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Icy Wind|spread:Pikachu,player-2,2",
            "miss|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,2|health:78/95",
            "damage|mon:Pikachu,player-2,2|health:83/100",
            "unboost|mon:Pikachu,player-2,2|stat:spe|by:1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn accuracy_check_only_once_for_multihit_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data, 453950743359796, pikachu_team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Fury Attack|noanim",
            "miss|mon:Pikachu,player-2,1",
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:86/95",
            "damage|mon:Pikachu,player-2,1|health:91/100",
            "animatemove|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:77/95",
            "damage|mon:Pikachu,player-2,1|health:82/100",
            "animatemove|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:68/95",
            "damage|mon:Pikachu,player-2,1|health:72/100",
            "animatemove|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:60/95",
            "damage|mon:Pikachu,player-2,1|health:64/100",
            "animatemove|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:51/95",
            "damage|mon:Pikachu,player-2,1|health:54/100",
            "hitcount|hits:5",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:42/95",
            "damage|mon:Pikachu,player-2,1|health:45/100",
            "animatemove|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:34/95",
            "damage|mon:Pikachu,player-2,1|health:36/100",
            "animatemove|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:25/95",
            "damage|mon:Pikachu,player-2,1|health:27/100",
            "hitcount|hits:3",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:16/95",
            "damage|mon:Pikachu,player-2,1|health:17/100",
            "animatemove|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:7/95",
            "damage|mon:Pikachu,player-2,1|health:8/100",
            "animatemove|mon:Pikachu,player-1,1|name:Fury Attack|target:Pikachu,player-2,1",
            "crit|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:0",
            "damage|mon:Pikachu,player-2,1|health:0",
            "faint|mon:Pikachu,player-2,1",
            "hitcount|hits:3",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn accuracy_check_for_multiaccuracy_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data, 21241564315, pikachu_team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Triple Kick|noanim",
            "miss|mon:Pikachu,player-2,1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:89/95",
            "damage|mon:Pikachu,player-2,1|health:94/100",
            "animatemove|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:78/95",
            "damage|mon:Pikachu,player-2,1|health:83/100",
            "animatemove|mon:Pikachu,player-1,1|name:Triple Kick|noanim",
            "miss|mon:Pikachu,player-2,1",
            "hitcount|hits:2",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:72/95",
            "damage|mon:Pikachu,player-2,1|health:76/100",
            "animatemove|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:61/95",
            "damage|mon:Pikachu,player-2,1|health:65/100",
            "animatemove|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
            "crit|mon:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:35/95",
            "damage|mon:Pikachu,player-2,1|health:37/100",
            "hitcount|hits:3",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Triple Kick|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:29/95",
            "damage|mon:Pikachu,player-2,1|health:31/100",
            "animatemove|mon:Pikachu,player-1,1|name:Triple Kick|noanim",
            "miss|mon:Pikachu,player-2,1",
            "hitcount|hits:1",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn moves_can_ignore_evasion() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data, 0, pikachu_team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 5"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 5"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 5"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Double Team|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Chip Away|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:54/95",
            "damage|mon:Pikachu,player-2,1|health:57/100",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Chip Away|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:16/95",
            "damage|mon:Pikachu,player-2,1|health:17/100",
            "residual",
            "turn|turn:8",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Chip Away|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:0",
            "damage|mon:Pikachu,player-2,1|health:0",
            "faint|mon:Pikachu,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
