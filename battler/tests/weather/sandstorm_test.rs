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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn golem() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Golem",
                    "species": "Golem",
                    "ability": "No Ability",
                    "moves": [
                        "Sandstorm",
                        "Confusion"
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

fn golem_with_smooth_rock() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Golem",
                    "species": "Golem",
                    "ability": "No Ability",
                    "moves": [
                        "Sandstorm",
                        "Confusion"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50,
                    "item": "Smooth Rock"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn golem_with_sand_stream() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Golem",
                    "species": "Golem",
                    "ability": "Sand Stream",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn steelix() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Steelix",
                    "species": "Steelix",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blastoise() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "No Ability",
                    "moves": [
                        "Confusion",
                        "Dig"
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

fn blastoise_with_utility_umbrella() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "No Ability",
                    "moves": [
                        "Water Gun"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50,
                    "item": "Utility Umbrella"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn rayquaza() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Rayquaza",
                    "species": "Rayquaza",
                    "ability": "Air Lock",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
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
        .with_controlled_rng(true)
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
fn sandstorm_lasts_for_five_turns() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, golem().unwrap(), blastoise().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Golem,player-1,1|name:Sandstorm",
            "weather|weather:Sandstorm",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:131/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:95/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:123/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:89/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Golem,player-1,1|name:Sandstorm|noanim",
            "fail|mon:Golem,player-1,1",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:115/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:83/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:107/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:77/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "weather|weather:Clear",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sandstorm_lasts_for_eight_turns_with_smooth_rock() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        golem_with_smooth_rock().unwrap(),
        blastoise().unwrap(),
    )
    .unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Golem,player-1,1|name:Sandstorm",
            "weather|weather:Sandstorm",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:131/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:95/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:123/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:89/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Golem,player-1,1|name:Sandstorm|noanim",
            "fail|mon:Golem,player-1,1",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:115/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:83/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:107/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:77/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:99/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:72/100",
            "residual",
            "turn|turn:6",
            ["time"],
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:91/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:66/100",
            "residual",
            "turn|turn:7",
            ["time"],
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:83/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:60/100",
            "residual",
            "turn|turn:8",
            ["time"],
            "weather|weather:Clear",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sandstorm_boosts_rock_special_defense() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, golem().unwrap(), blastoise().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-2,1|name:Confusion|target:Golem,player-1,1",
            "split|side:0",
            "damage|mon:Golem,player-1,1|health:111/140",
            "damage|mon:Golem,player-1,1|health:80/100",
            "move|mon:Golem,player-1,1|name:Confusion|target:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:127/139",
            "damage|mon:Blastoise,player-2,1|health:92/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Golem,player-1,1|name:Sandstorm",
            "weather|weather:Sandstorm",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:119/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:86/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-2,1|name:Confusion|target:Golem,player-1,1",
            "split|side:0",
            "damage|mon:Golem,player-1,1|health:93/140",
            "damage|mon:Golem,player-1,1|health:67/100",
            "move|mon:Golem,player-1,1|name:Confusion|target:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:107/139",
            "damage|mon:Blastoise,player-2,1|health:77/100",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:99/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:72/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn steel_types_resist_sandstorm() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, golem().unwrap(), steelix().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Golem,player-1,1|name:Sandstorm",
            "weather|weather:Sandstorm",
            "weather|weather:Sandstorm|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sand_stream_starts_sandstorm_on_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        golem_with_sand_stream().unwrap(),
        blastoise().unwrap(),
    )
    .unwrap();
    assert_eq!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Sandstorm|from:ability:Sand Stream|of:Golem,player-1,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn air_lock_suppresses_sandstorm() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, golem().unwrap(), rayquaza().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "ability|mon:Rayquaza,player-2,1|ability:Air Lock",
            "turn|turn:1",
            ["time"],
            "move|mon:Golem,player-1,1|name:Sandstorm",
            "weather|weather:Sandstorm",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn utility_umbrella_does_not_suppress_sandstorm() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        golem().unwrap(),
        blastoise_with_utility_umbrella().unwrap(),
    )
    .unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Golem,player-1,1|name:Sandstorm",
            "weather|weather:Sandstorm",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:131/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:95/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dig_is_protected_from_residual_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, golem().unwrap(), blastoise().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-2,1|name:Dig|noanim",
            "prepare|mon:Blastoise,player-2,1|move:Dig",
            "move|mon:Golem,player-1,1|name:Sandstorm",
            "weather|weather:Sandstorm",
            "weather|weather:Sandstorm|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-2,1|name:Dig|target:Golem,player-1,1",
            "supereffective|mon:Golem,player-1,1",
            "split|side:0",
            "damage|mon:Golem,player-1,1|health:94/140",
            "damage|mon:Golem,player-1,1|health:68/100",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:131/139",
            "damage|mon:Blastoise,player-2,1|from:weather:Sandstorm|health:95/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
