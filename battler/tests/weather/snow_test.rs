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
    LogMatch,
    TestBattleBuilder,
};

fn dewgong() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Dewgong",
                    "species": "Dewgong",
                    "ability": "No Ability",
                    "moves": [
                        "Snowscape",
                        "Slash"
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

fn dewgong_with_icy_rock() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Dewgong",
                    "species": "Dewgong",
                    "ability": "No Ability",
                    "moves": [
                        "Snowscape",
                        "Slash"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50,
                    "item": "Icy Rock"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn dewgong_with_snow_warning() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Dewgong",
                    "species": "Dewgong",
                    "ability": "Snow Warning",
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

fn blastoise() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "No Ability",
                    "moves": [
                        "Slash"
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
fn snow_lasts_for_five_turns() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, dewgong().unwrap(), blastoise().unwrap()).unwrap();
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
            "move|mon:Dewgong,player-1,1|name:Snowscape",
            "weather|weather:Snow",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Dewgong,player-1,1|name:Snowscape|noanim",
            "fail|mon:Dewgong,player-1,1",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Snow|residual",
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
fn snow_lasts_for_eight_turns_with_icy_rock() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        dewgong_with_icy_rock().unwrap(),
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
            "move|mon:Dewgong,player-1,1|name:Snowscape",
            "weather|weather:Snow",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Dewgong,player-1,1|name:Snowscape|noanim",
            "fail|mon:Dewgong,player-1,1",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:5",
            ["time"],
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:6",
            ["time"],
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:7",
            ["time"],
            "weather|weather:Snow|residual",
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
fn snow_boosts_ice_defense() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 345332, dewgong().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-2,1|name:Slash|target:Dewgong,player-1,1",
            "split|side:0",
            "damage|mon:Dewgong,player-1,1|health:118/150",
            "damage|mon:Dewgong,player-1,1|health:79/100",
            "move|mon:Dewgong,player-1,1|name:Slash|target:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:119/139",
            "damage|mon:Blastoise,player-2,1|health:86/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Dewgong,player-1,1|name:Snowscape",
            "weather|weather:Snow",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Blastoise,player-2,1|name:Slash|target:Dewgong,player-1,1",
            "split|side:0",
            "damage|mon:Dewgong,player-1,1|health:98/150",
            "damage|mon:Dewgong,player-1,1|health:66/100",
            "move|mon:Dewgong,player-1,1|name:Slash|target:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:96/139",
            "damage|mon:Blastoise,player-2,1|health:70/100",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn snow_warning_starts_snow_on_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        dewgong_with_snow_warning().unwrap(),
        blastoise().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Snow|from:ability:Snow Warning|of:Dewgong,player-1,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
