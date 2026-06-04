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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Terapagos",
                    "species": "Terapagos",
                    "ability": "Tera Shift",
                    "moves": [
                        "Splash",
                        "Rain Dance",
                        "Electric Terrain"
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
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn teraform_zero_removes_weather_and_terrain() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Terapagos,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "move|mon:Terapagos,player-2,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "tera|mon:Terapagos,player-1,1|type:Stellar",
            "split|side:0",
            ["specieschange", "player-1", "tera:Stellar", "species:Terapagos-Stellar"],
            ["specieschange", "player-1", "tera:Stellar", "species:Terapagos-Stellar"],
            "formechange|mon:Terapagos,player-1,1|species:Terapagos-Stellar|from:species:Terapagos-Terastal",
            "ability|mon:Terapagos,player-1,1|ability:Teraform Zero",
            "clearweather",
            "fieldend|move:Electric Terrain",
            "move|mon:Terapagos,player-1,1|name:Splash|target:Terapagos,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
