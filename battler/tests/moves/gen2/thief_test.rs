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

fn crobat() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Crobat",
                    "species": "Crobat",
                    "ability": "No Ability",
                    "moves": [
                        "Thief"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn crobat_with_goggles() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Crobat",
                    "species": "Crobat",
                    "ability": "No Ability",
                    "moves": [
                        "Thief"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Safety Goggles"
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
        .with_weather(Some("sandstormweather".to_string()))
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn thief_steals_target_item() {
    let mut battle = make_battle(0, crobat().unwrap(), crobat_with_goggles().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "weather|weather:Sandstorm|residual",
            "split|side:0",
            "damage|mon:Crobat,player-1,1|from:weather:Sandstorm|health:136/145",
            "damage|mon:Crobat,player-1,1|from:weather:Sandstorm|health:94/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Crobat,player-1,1|name:Thief|target:Crobat,player-2,1",
            "split|side:1",
            "damage|mon:Crobat,player-2,1|health:115/145",
            "damage|mon:Crobat,player-2,1|health:80/100",
            "itemend|mon:Crobat,player-2,1|item:Safety Goggles|silent|from:move:Thief|of:Crobat,player-1,1",
            "item|mon:Crobat,player-1,1|item:Safety Goggles|from:move:Thief|of:Crobat,player-2,1",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Crobat,player-2,1|from:weather:Sandstorm|health:106/145",
            "damage|mon:Crobat,player-2,1|from:weather:Sandstorm|health:74/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
