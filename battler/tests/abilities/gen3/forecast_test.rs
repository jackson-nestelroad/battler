use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn castform() -> Result<TeamData> {
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
                        "Sandstorm",
                        "Fling"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn opponents() -> Result<TeamData> {
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
                        "Aurora Beam",
                        "Trick",
                        "Embargo",
                        "Gastro Acid",
                        "Worry Seed"
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

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn forecast_transforms_castform_in_rain() {
    let mut battle = make_battle(0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
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
    let mut battle = make_battle(0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Castform,player-1,1|species:Castform-Sunny|from:ability:Forecast",
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
    let mut battle = make_battle(0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Hail",
            "weather|weather:Hail",
            "formechange|mon:Castform,player-1,1|species:Castform-Snowy|from:ability:Forecast",
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
    let mut battle = make_battle(0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Snowscape",
            "weather|weather:Snow",
            "formechange|mon:Castform,player-1,1|species:Castform-Snowy|from:ability:Forecast",
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
    let mut battle = make_battle(0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-2,1|name:Transform|target:Castform,player-1,1",
            "transform|mon:Ditto,player-2,1|into:Castform,player-1,1|species:Castform",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
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
    let mut battle = make_battle(0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Rayquaza"],
            ["switch", "player-2", "Rayquaza"],
            "ability|mon:Rayquaza,player-2,1|ability:Air Lock",
            "formechange|mon:Castform,player-1,1|species:Castform|from:ability:Forecast",
            "residual",
            "turn|turn:3",
            ["time"],
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
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

#[test]
fn forecast_reverts_due_to_gaining_weather_suppressing_item() {
    let mut opponents = opponents().unwrap();
    opponents.members[0].item = Some("Utility Umbrella".to_owned());
    let mut battle = make_battle(0, castform().unwrap(), opponents).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-2,1|name:Trick|target:Castform,player-1,1",
            "itemend|mon:Ditto,player-2,1|item:Utility Umbrella|from:move:Trick",
            "item|mon:Castform,player-1,1|item:Utility Umbrella|from:move:Trick|of:Ditto,player-2,1",
            "formechange|mon:Castform,player-1,1|species:Castform|from:ability:Forecast|of:Ditto,player-2,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Castform,player-1,1|name:Fling|target:Ditto,player-2,1",
            "activate|mon:Castform,player-1,1|move:Fling|item:Utility Umbrella",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:71/108",
            "damage|mon:Ditto,player-2,1|health:66/100",
            "itemend|mon:Castform,player-1,1|item:Utility Umbrella|silent|from:move:Fling",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forecast_activates_due_to_suppressing_weather_suppressing_item() {
    let mut opponents = opponents().unwrap();
    opponents.members[0].item = Some("Utility Umbrella".to_owned());
    let mut battle = make_battle(0, castform().unwrap(), opponents).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-2,1|name:Trick|target:Castform,player-1,1",
            "itemend|mon:Ditto,player-2,1|item:Utility Umbrella|from:move:Trick",
            "item|mon:Castform,player-1,1|item:Utility Umbrella|from:move:Trick|of:Ditto,player-2,1",
            "formechange|mon:Castform,player-1,1|species:Castform|from:ability:Forecast|of:Ditto,player-2,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Ditto,player-2,1|name:Embargo|target:Castform,player-1,1",
            "start|mon:Castform,player-1,1|move:Embargo",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast|of:Ditto,player-2,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forecast_reverts_due_to_weather_suppressing_move() {
    let mut battle = make_battle(0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 6"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-2,1|name:Gastro Acid|target:Castform,player-1,1",
            "abilityend|mon:Castform,player-1,1|ability:Forecast|from:move:Gastro Acid|of:Ditto,player-2,1",
            "formechange|mon:Castform,player-1,1|species:Castform|from:ability:Forecast|of:Ditto,player-2,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forecast_reverts_due_to_losing_ability() {
    let mut battle = make_battle(0, castform().unwrap(), opponents().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 7"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Castform,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Castform,player-1,1|species:Castform-Rainy|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-2,1|name:Worry Seed|target:Castform,player-1,1",
            "abilityend|mon:Castform,player-1,1|ability:Forecast|from:move:Worry Seed|of:Ditto,player-2,1",
            "formechange|mon:Castform,player-1,1|species:Castform|from:ability:Forecast|of:Ditto,player-2,1",
            "ability|mon:Castform,player-1,1|ability:Insomnia|from:move:Worry Seed|of:Ditto,player-2,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
