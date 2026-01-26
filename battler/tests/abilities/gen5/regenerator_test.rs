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

fn mienshao() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mienshao",
                    "species": "Mienshao",
                    "ability": "Regenerator",
                    "moves": [
                        "Focus Blast"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Mienshao",
                    "species": "Mienshao",
                    "ability": "Regenerator",
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
fn regenerator_heals_on_switch_out() {
    let mut battle = make_battle(0, mienshao().unwrap(), mienshao().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mienshao,player-1,1|name:Focus Blast|target:Mienshao,player-2,1",
            "split|side:1",
            "damage|mon:Mienshao,player-2,1|health:5/125",
            "damage|mon:Mienshao,player-2,1|health:4/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            "heal|mon:Mienshao,player-2,1|from:ability:Regenerator|health:46/125",
            "heal|mon:Mienshao,player-2,1|from:ability:Regenerator|health:37/100",
            "split|side:1",
            ["switch"],
            ["switch"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
