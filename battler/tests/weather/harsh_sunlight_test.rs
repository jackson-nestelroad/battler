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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn charizard() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Sunny Day",
                        "Flamethrower",
                        "Solar Beam",
                        "Growth"
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

fn charizard_with_heat_rock() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Sunny Day",
                        "Flamethrower",
                        "Solar Beam"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50,
                    "item": "Heat Rock"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn charizard_with_drought() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "Drought",
                    "moves": [
                        "Sunny Day",
                        "Flamethrower",
                        "Solar Beam"
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

fn blastoise() -> Result<TeamData, Error> {
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
                    "level": 50
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
                    "moves": [
                        "Flamethrower"
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
fn harsh_sunlight_lasts_five_turns() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, charizard().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Sunny Day|noanim",
            "fail|mon:Charizard,player-1,1",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
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
fn harsh_sunlight_lasts_eight_turns_with_heat_rock() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        charizard_with_heat_rock().unwrap(),
        blastoise().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Sunny Day|noanim",
            "fail|mon:Charizard,player-1,1",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:5",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:6",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:7",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
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
fn harsh_sunlight_boosts_fire_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, charizard().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-1,1|name:Flamethrower|target:Blastoise,player-2,1",
            "resisted|mon:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:109/139",
            "damage|mon:Blastoise,player-2,1|health:79/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Flamethrower|target:Blastoise,player-2,1",
            "resisted|mon:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:66/139",
            "damage|mon:Blastoise,player-2,1|health:48/100",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn harsh_sunlight_reduces_water_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, charizard().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-2,1|name:Water Gun|target:Charizard,player-1,1",
            "supereffective|mon:Charizard,player-1,1",
            "split|side:0",
            "damage|mon:Charizard,player-1,1|health:84/138",
            "damage|mon:Charizard,player-1,1|health:61/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-2,1|name:Water Gun|target:Charizard,player-1,1",
            "supereffective|mon:Charizard,player-1,1",
            "split|side:0",
            "damage|mon:Charizard,player-1,1|health:60/138",
            "damage|mon:Charizard,player-1,1|health:44/100",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn harsh_sunlight_removes_charge_turn_from_solar_beam() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, charizard().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Solar Beam|noanim",
            "prepare|mon:Charizard,player-1,1|move:Solar Beam",
            "animatemove|mon:Charizard,player-1,1|name:Solar Beam",
            "supereffective|mon:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:31/139",
            "damage|mon:Blastoise,player-2,1|health:23/100",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn drought_starts_harsh_sunlight_on_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        charizard_with_drought().unwrap(),
        blastoise().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Harsh Sunlight|from:ability:Drought|of:Charizard,player-1,1",
            "turn|turn:1",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:5",
            ["time"],
            "weather|weather:Clear",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn air_lock_suppresses_harsh_sunlight() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, charizard().unwrap(), rayquaza().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
            "move|mon:Charizard,player-1,1|name:Flamethrower|target:Rayquaza,player-2,1",
            "resisted|mon:Rayquaza,player-2,1",
            "split|side:1",
            "damage|mon:Rayquaza,player-2,1|health:130/165",
            "damage|mon:Rayquaza,player-2,1|health:79/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Flamethrower|target:Rayquaza,player-2,1",
            "resisted|mon:Rayquaza,player-2,1",
            "split|side:1",
            "damage|mon:Rayquaza,player-2,1|health:97/165",
            "damage|mon:Rayquaza,player-2,1|health:59/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Solar Beam|noanim",
            "prepare|mon:Charizard,player-1,1|move:Solar Beam",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn harsh_sunlight_increases_growth_boost() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, charizard().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-1,1|name:Growth|target:Charizard,player-1,1",
            "boost|mon:Charizard,player-1,1|stat:atk|by:1",
            "boost|mon:Charizard,player-1,1|stat:spa|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Growth|target:Charizard,player-1,1",
            "boost|mon:Charizard,player-1,1|stat:atk|by:2",
            "boost|mon:Charizard,player-1,1|stat:spa|by:2",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
