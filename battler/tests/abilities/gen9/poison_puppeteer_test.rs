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
                    "name": "Pecharunt",
                    "species": "Pecharunt",
                    "ability": "Poison Puppeteer",
                    "moves": [
                        "Toxic"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
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
fn poison_puppeteer_inflicts_confusion_with_poison() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Terapagos"],
            ["switch", "player-2", "Terapagos"],
            "activate|mon:Terapagos,player-2,1|ability:Tera Shift",
            "split|side:1",
            ["specieschange", "player-2", "Terapagos-Terastal"],
            ["specieschange", "player-2", "Terapagos-Terastal"],
            "formechange|mon:Terapagos,player-2,1|species:Terapagos-Terastal|from:ability:Tera Shift",
            "move|mon:Pecharunt,player-1,1|name:Toxic|target:Terapagos,player-2,1",
            "status|mon:Terapagos,player-2,1|status:Bad Poison",
            "start|mon:Terapagos,player-2,1|condition:Confusion|from:ability:Poison Puppeteer|of:Pecharunt,player-1,1",
            "split|side:1",
            "damage|mon:Terapagos,player-2,1|from:status:Bad Poison|health:146/155",
            "damage|mon:Terapagos,player-2,1|from:status:Bad Poison|health:95/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
