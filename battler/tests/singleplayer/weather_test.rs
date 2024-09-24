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
    LogMatch,
    TestBattleBuilder,
};

fn pikachu() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Sunny Day"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_trainer_singles_battle(
    data: &dyn DataStore,
    seed: u64,
    weather: String,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_weather(Some(weather))
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_player_to_side_2("trainer", "Trainer")
        .with_team("protagonist", team_1)
        .with_team("trainer", team_2)
        .build(data)
}

#[test]
fn battle_starts_with_rain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_trainer_singles_battle(
        &data,
        0,
        "rainweather".to_owned(),
        pikachu().unwrap(),
        pikachu().unwrap(),
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
            "weather|weather:Rain|from:Start",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn battle_starts_with_harsh_sunlight() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_trainer_singles_battle(
        &data,
        0,
        "harshsunlight".to_owned(),
        pikachu().unwrap(),
        pikachu().unwrap(),
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
            "weather|weather:Harsh Sunlight|from:Start",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn battle_starts_with_sandstorm() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_trainer_singles_battle(
        &data,
        0,
        "sandstormweather".to_owned(),
        pikachu().unwrap(),
        pikachu().unwrap(),
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
            "weather|weather:Sandstorm|from:Start",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn battle_starts_with_hail() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_trainer_singles_battle(
        &data,
        0,
        "hailweather".to_owned(),
        pikachu().unwrap(),
        pikachu().unwrap(),
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
            "weather|weather:Hail|from:Start",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn battle_starts_with_snow() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_trainer_singles_battle(
        &data,
        0,
        "snowweather".to_owned(),
        pikachu().unwrap(),
        pikachu().unwrap(),
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
            "weather|weather:Snow|from:Start",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn battle_goes_back_to_default_weather() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_trainer_singles_battle(
        &data,
        0,
        "rainweather".to_owned(),
        pikachu().unwrap(),
        pikachu().unwrap(),
    )
    .unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("protagonist", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("trainer", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Rain|from:Start",
            "turn|turn:1",
            ["time"],
            "move|mon:Pikachu,protagonist,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
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
            "weather|weather:Rain|from:Start",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
