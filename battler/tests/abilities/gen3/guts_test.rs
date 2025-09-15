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

fn swellow() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Swellow",
                    "species": "Swellow",
                    "ability": "Guts",
                    "moves": [
                        "Tackle",
                        "Will-O-Wisp"
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn guts_increases_attack_with_status() {
    let mut battle = make_battle(
        0,
        swellow().unwrap(),
        swellow().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swellow,player-1,1|name:Tackle|target:Swellow,player-2,1",
            "split|side:1",
            "damage|mon:Swellow,player-2,1|health:83/120",
            "damage|mon:Swellow,player-2,1|health:70/100",
            "move|mon:Swellow,player-2,1|name:Will-O-Wisp|target:Swellow,player-1,1",
            "status|mon:Swellow,player-1,1|status:Burn",
            "split|side:0",
            "damage|mon:Swellow,player-1,1|from:status:Burn|health:113/120",
            "damage|mon:Swellow,player-1,1|from:status:Burn|health:95/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Swellow,player-1,1|name:Tackle|target:Swellow,player-2,1",
            "split|side:1",
            "damage|mon:Swellow,player-2,1|health:16/120",
            "damage|mon:Swellow,player-2,1|health:14/100",
            "split|side:0",
            "damage|mon:Swellow,player-1,1|from:status:Burn|health:106/120",
            "damage|mon:Swellow,player-1,1|from:status:Burn|health:89/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
