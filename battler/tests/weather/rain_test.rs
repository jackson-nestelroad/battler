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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn blastoise() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "No Ability",
                    "moves": [
                        "Rain Dance",
                        "Water Gun",
                        "Thunder",
                        "Embargo"
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

fn blastoise_with_damp_rock() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "No Ability",
                    "moves": [
                        "Rain Dance",
                        "Water Gun",
                        "Thunder",
                        "Embargo"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50,
                    "item": "Damp Rock"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blastoise_with_drizzle() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Drizzle",
                    "moves": [
                        "Rain Dance",
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

fn charizard() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Flamethrower",
                        "Double Team"
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

fn charizard_with_utility_umbrella() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Flamethrower",
                        "Double Team"
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

fn rayquaza() -> Result<TeamData> {
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

fn rayquaza_kyogre() -> Result<TeamData> {
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
                },
                {
                    "name": "Kyogre",
                    "species": "Kyogre",
                    "ability": "Drizzle",
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
) -> Result<PublicCoreBattle> {
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
fn rain_lasts_for_five_turns() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, blastoise().unwrap(), charizard().unwrap()).unwrap();
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
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Rain Dance|noanim",
            "fail|mon:Blastoise,player-1,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:5",
            ["time"],
            "clearweather",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rain_lasts_for_eight_turns_with_damp_rock() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        blastoise_with_damp_rock().unwrap(),
        charizard().unwrap(),
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
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Rain Dance|noanim",
            "fail|mon:Blastoise,player-1,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:5",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:6",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:7",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:8",
            ["time"],
            "clearweather",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rain_boosts_water_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, blastoise().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-1,1|name:Water Gun|target:Charizard,player-2,1",
            "supereffective|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:84/138",
            "damage|mon:Charizard,player-2,1|health:61/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Water Gun|target:Charizard,player-2,1",
            "supereffective|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:10/138",
            "damage|mon:Charizard,player-2,1|health:8/100",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rain_reduces_fire_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, blastoise().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Flamethrower|target:Blastoise,player-1,1",
            "resisted|mon:Blastoise,player-1,1",
            "split|side:0",
            "damage|mon:Blastoise,player-1,1|health:109/139",
            "damage|mon:Blastoise,player-1,1|health:79/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Flamethrower|target:Blastoise,player-1,1",
            "resisted|mon:Blastoise,player-1,1",
            "split|side:0",
            "damage|mon:Blastoise,player-1,1|health:95/139",
            "damage|mon:Blastoise,player-1,1|health:69/100",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rain_increases_thunder_accuracy() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, blastoise().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Double Team|target:Charizard,player-2,1",
            "boost|mon:Charizard,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Double Team|target:Charizard,player-2,1",
            "boost|mon:Charizard,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Double Team|target:Charizard,player-2,1",
            "boost|mon:Charizard,player-2,1|stat:eva|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Thunder|target:Charizard,player-2,1",
            "supereffective|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:52/138",
            "damage|mon:Charizard,player-2,1|health:38/100",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn drizzle_starts_rain_on_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        blastoise_with_drizzle().unwrap(),
        charizard().unwrap(),
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
            "weather|weather:Rain|from:ability:Drizzle|of:Blastoise,player-1,1",
            "turn|turn:1",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:5",
            ["time"],
            "clearweather",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn air_lock_suppresses_rain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, blastoise().unwrap(), rayquaza().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    get_controlled_rng_for_battle(&mut battle)
        .unwrap()
        .insert_fake_values_relative_to_sequence_count([(1, 99)]);
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
            "move|mon:Blastoise,player-1,1|name:Water Gun|target:Rayquaza,player-2,1",
            "resisted|mon:Rayquaza,player-2,1",
            "split|side:1",
            "damage|mon:Rayquaza,player-2,1|health:153/165",
            "damage|mon:Rayquaza,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Water Gun|target:Rayquaza,player-2,1",
            "resisted|mon:Rayquaza,player-2,1",
            "split|side:1",
            "damage|mon:Rayquaza,player-2,1|health:141/165",
            "damage|mon:Rayquaza,player-2,1|health:86/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Thunder|noanim",
            "miss|mon:Rayquaza,player-2,1",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn rain_finishes_normally_with_air_lock() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 0, blastoise().unwrap(), rayquaza_kyogre().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

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
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

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
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "residual",
            "turn|turn:2",
            ["time"],
            "residual",
            "turn|turn:3",
            ["time"],
            "residual",
            "turn|turn:4",
            ["time"],
            "residual",
            "turn|turn:5",
            ["time"],
            "clearweather",
            "residual",
            "turn|turn:6",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Kyogre"],
            ["switch", "player-2", "Kyogre"],
            "weather|weather:Rain|from:ability:Drizzle|of:Kyogre,player-2,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn utility_umbrella_suppresses_rain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        blastoise().unwrap(),
        charizard_with_utility_umbrella().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    get_controlled_rng_for_battle(&mut battle)
        .unwrap()
        .insert_fake_values_relative_to_sequence_count([(1, 99)]);
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-1,1|name:Water Gun|target:Charizard,player-2,1",
            "supereffective|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:84/138",
            "damage|mon:Charizard,player-2,1|health:61/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Water Gun|target:Charizard,player-2,1",
            "supereffective|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:34/138",
            "damage|mon:Charizard,player-2,1|health:25/100",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Thunder|noanim",
            "miss|mon:Charizard,player-2,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn suppressed_utility_umbrella_does_not_suppress_rain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        blastoise().unwrap(),
        charizard_with_utility_umbrella().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-1,1|name:Embargo|target:Charizard,player-2,1",
            "start|mon:Charizard,player-2,1|move:Embargo",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Water Gun|target:Charizard,player-2,1",
            "supereffective|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:84/138",
            "damage|mon:Charizard,player-2,1|health:61/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Water Gun|target:Charizard,player-2,1",
            "supereffective|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:10/138",
            "damage|mon:Charizard,player-2,1|health:8/100",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
