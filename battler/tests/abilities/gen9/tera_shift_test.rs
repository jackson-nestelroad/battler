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
    assert_logs_since_start_eq,
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
fn tera_shift_transforms_terapagos() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "species:Terapagos|"],
            ["switch", "player-1", "species:Terapagos|"],
            "split|side:1",
            ["switch", "player-2", "species:Terapagos|"],
            ["switch", "player-2", "species:Terapagos|"],
            "activate|mon:Terapagos,player-1,1|ability:Tera Shift",
            "split|side:0",
            ["specieschange", "player-1", "species:Terapagos-Terastal|"],
            ["specieschange", "player-1", "species:Terapagos-Terastal|"],
            "formechange|mon:Terapagos,player-1,1|species:Terapagos-Terastal|from:ability:Tera Shift",
            "activate|mon:Terapagos,player-2,1|ability:Tera Shift",
            "split|side:1",
            ["specieschange", "player-2", "species:Terapagos-Terastal|"],
            ["specieschange", "player-2", "species:Terapagos-Terastal|"],
            "formechange|mon:Terapagos,player-2,1|species:Terapagos-Terastal|from:ability:Tera Shift",
            "turn|turn:1",
            "continue",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
