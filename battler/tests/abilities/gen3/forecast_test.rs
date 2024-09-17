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

fn castform() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Castform",
                    "species": "Castform",
                    "ability": "Forecast",
                    "moves": [
                        "Rain Dance",
                        "Sunny Day",
                        "Hail",
                        "Snowscape",
                        "Sandstorm"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn opponents() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "Limber",
                    "moves": [
                        "Transform",
                        "Thunder Shock",
                        "Vine Whip",
                        "Aurora Beam"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Rayquaza",
                    "species": "Rayquaza",
                    "ability": "Air Lock",
                    "moves": [],
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
fn forecast_transforms_castform_in_rain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "split|side:0",
            ["specieschange", "player-1", "name:Castform", "species:Castform-Rainy"],
            ["specieschange", "player-1", "name:Castform", "species:Castform-Rainy"],
            "formechange|mon:Castform,player-1,1|from:ability:Forecast",
            "move|mon:Ditto,player-2,1|name:Thunder Shock|target:Castform,player-1,1",
            "supereffective|mon:Castform,player-1,1",
            "split|side:0",
            "damage|mon:Castform,player-1,1|health:104/130",
            "damage|mon:Castform,player-1,1|health:80/100",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forecast_transforms_castform_in_harsh_sunlight() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "split|side:0",
            ["specieschange", "player-1", "name:Castform", "species:Castform-Sunny"],
            ["specieschange", "player-1", "name:Castform", "species:Castform-Sunny"],
            "formechange|mon:Castform,player-1,1|from:ability:Forecast",
            "move|mon:Ditto,player-2,1|name:Vine Whip|target:Castform,player-1,1",
            "resisted|mon:Castform,player-1,1",
            "split|side:0",
            "damage|mon:Castform,player-1,1|health:123/130",
            "damage|mon:Castform,player-1,1|health:95/100",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forecast_transforms_castform_in_hail() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Hail",
            "weather|weather:Hail",
            "split|side:0",
            ["specieschange", "player-1", "name:Castform", "species:Castform-Snowy"],
            ["specieschange", "player-1", "name:Castform", "species:Castform-Snowy"],
            "formechange|mon:Castform,player-1,1|from:ability:Forecast",
            "move|mon:Ditto,player-2,1|name:Aurora Beam|target:Castform,player-1,1",
            "resisted|mon:Castform,player-1,1",
            "split|side:0",
            "damage|mon:Castform,player-1,1|health:120/130",
            "damage|mon:Castform,player-1,1|health:93/100",
            "weather|weather:Hail|residual",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|from:weather:Hail|health:102/108",
            "damage|mon:Ditto,player-2,1|from:weather:Hail|health:95/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forecast_transforms_castform_in_snow() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Snowscape",
            "weather|weather:Snow",
            "split|side:0",
            ["specieschange", "player-1", "name:Castform", "species:Castform-Snowy"],
            ["specieschange", "player-1", "name:Castform", "species:Castform-Snowy"],
            "formechange|mon:Castform,player-1,1|from:ability:Forecast",
            "move|mon:Ditto,player-2,1|name:Aurora Beam|target:Castform,player-1,1",
            "resisted|mon:Castform,player-1,1",
            "split|side:0",
            "damage|mon:Castform,player-1,1|health:120/130",
            "damage|mon:Castform,player-1,1|health:93/100",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forecast_fails_for_transformed_ditto() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-2,1|name:Transform|target:Castform,player-1,1",
            "transform|mon:Ditto,player-2,1|into:Castform,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "split|side:0",
            ["specieschange", "player-1", "name:Castform", "species:Castform-Rainy"],
            ["specieschange", "player-1", "name:Castform", "species:Castform-Rainy"],
            "formechange|mon:Castform,player-1,1|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forecast_reverts_due_to_suppressed_weather() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "switch 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "split|side:0",
            ["specieschange", "player-1", "name:Castform", "species:Castform-Rainy"],
            ["specieschange", "player-1", "name:Castform", "species:Castform-Rainy"],
            "formechange|mon:Castform,player-1,1|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Rayquaza"],
            ["switch", "player-2", "Rayquaza"],
            "ability|mon:Rayquaza,player-2,1|ability:Air Lock",
            "split|side:0",
            ["specieschange", "player-1", "name:Castform", "species:Castform"],
            ["specieschange", "player-1", "name:Castform", "species:Castform"],
            "formechange|mon:Castform,player-1,1|from:ability:Forecast",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            ["specieschange", "player-1", "name:Castform", "species:Castform-Rainy"],
            ["specieschange", "player-1", "name:Castform", "species:Castform-Rainy"],
            "formechange|mon:Castform,player-1,1|from:ability:Forecast",
            "split|side:1",
            ["switch", "player-2", "Ditto"],
            ["switch", "player-2", "Ditto"],
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}