use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn team_1() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cherrim",
                    "species": "Cherrim",
                    "ability": "Flower Gift",
                    "moves": [
                        "Sunny Day",
                        "Rain Dance",
                        "Vine Whip"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Luxray",
                    "species": "Luxray",
                    "ability": "No Ability",
                    "moves": [
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
fn team_2() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Luxray",
                    "species": "Luxray",
                    "ability": "No Ability",
                    "item": "Utility Umbrella",
                    "moves": [
                        "Thunderbolt",
                        "Trick"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "No Ability",
                    "moves": [
                        "Transform"
                    ],
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
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn flower_gift_changes_cherrim_form_in_sun() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cherrim,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim|from:ability:Flower Gift",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_gift_boosts_attack_in_sun() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cherrim,player-1,1|name:Vine Whip|target:Luxray,player-2,1",
            "split|side:1",
            "damage|mon:Luxray,player-2,1|health:115/140",
            "damage|mon:Luxray,player-2,1|health:83/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cherrim,player-1,1|name:Vine Whip|target:Luxray,player-2,1",
            "split|side:1",
            "damage|mon:Luxray,player-2,1|health:79/140",
            "damage|mon:Luxray,player-2,1|health:57/100",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_gift_boosts_ally_attack_in_sun() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Luxray,player-1,2|name:Tackle|target:Luxray,player-2,1",
            "split|side:1",
            "damage|mon:Luxray,player-2,1|health:112/140",
            "damage|mon:Luxray,player-2,1|health:80/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Luxray,player-1,2|name:Tackle|target:Luxray,player-2,1",
            "split|side:1",
            "damage|mon:Luxray,player-2,1|health:71/140",
            "damage|mon:Luxray,player-2,1|health:51/100",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_gift_boosts_special_defense_in_sun() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Luxray,player-2,1|name:Thunderbolt|target:Cherrim,player-1,1",
            "resisted|mon:Cherrim,player-1,1",
            "split|side:0",
            "damage|mon:Cherrim,player-1,1|health:94/130",
            "damage|mon:Cherrim,player-1,1|health:73/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Luxray,player-2,1|name:Thunderbolt|target:Cherrim,player-1,1",
            "resisted|mon:Cherrim,player-1,1",
            "split|side:0",
            "damage|mon:Cherrim,player-1,1|health:70/130",
            "damage|mon:Cherrim,player-1,1|health:54/100",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_gift_boosts_ally_special_defense_in_sun() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Luxray,player-2,1|name:Thunderbolt|target:Luxray,player-1,2",
            "resisted|mon:Luxray,player-1,2",
            "split|side:0",
            "damage|mon:Luxray,player-1,2|health:104/140",
            "damage|mon:Luxray,player-1,2|health:75/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Luxray,player-2,1|name:Thunderbolt|target:Luxray,player-1,2",
            "resisted|mon:Luxray,player-1,2",
            "split|side:0",
            "damage|mon:Luxray,player-1,2|health:80/140",
            "damage|mon:Luxray,player-1,2|health:58/100",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_gift_does_not_activate_when_transformed() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-2,2|name:Transform|target:Cherrim,player-1,1",
            "transform|mon:Ditto,player-2,2|into:Cherrim,player-1,1|species:Cherrim",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_gift_ends_due_to_gaining_weather_suppressing_item() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Luxray,player-2,1|name:Trick|target:Cherrim,player-1,1",
            "itemend|mon:Luxray,player-2,1|item:Utility Umbrella|from:move:Trick",
            "item|mon:Cherrim,player-1,1|item:Utility Umbrella|from:move:Trick|of:Luxray,player-2,1",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_gift_does_not_activate_when_weather_is_suppressed() {
    let mut team_2 = team_2().unwrap();
    team_2.members[0].ability = "Cloud Nine".to_owned();
    let mut battle = make_battle(0, team_1().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_gift_activates_when_weather_is_unsuppressed() {
    let mut team_2 = team_2().unwrap();
    team_2.members[1].ability = "Cloud Nine".to_owned();
    let mut battle = make_battle(0, team_1().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Ditto,player-2,2|name:Transform|target:Cherrim,player-1,1",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "transform|mon:Ditto,player-2,2|into:Cherrim,player-1,1|species:Cherrim",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn can_transform_into_cherrim_sunshine() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cherrim,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim-Sunshine|from:ability:Flower Gift",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Ditto,player-2,2|name:Transform|target:Cherrim,player-1,1",
            "transform|mon:Ditto,player-2,2|into:Cherrim,player-1,1|species:Cherrim-Sunshine",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cherrim,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "formechange|mon:Cherrim,player-1,1|species:Cherrim|from:ability:Flower Gift",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
