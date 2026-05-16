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
                    "name": "Toucannon",
                    "species": "Toucannon",
                    "ability": "No Ability",
                    "moves": [
                        "Beak Blast",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
fn beak_blast_burns_attackers_while_before_use() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Toucannon,player-1,1|move:Beak Blast",
            "move|mon:Toucannon,player-2,1|name:Tackle|target:Toucannon,player-1,1",
            "split|side:0",
            "damage|mon:Toucannon,player-1,1|health:191/270",
            "damage|mon:Toucannon,player-1,1|health:71/100",
            "status|mon:Toucannon,player-2,1|status:Burn|from:move:Beak Blast|of:Toucannon,player-1,1",
            "move|mon:Toucannon,player-1,1|name:Beak Blast|target:Toucannon,player-2,1",
            "split|side:1",
            "damage|mon:Toucannon,player-2,1|health:90/270",
            "damage|mon:Toucannon,player-2,1|health:34/100",
            "split|side:1",
            "damage|mon:Toucannon,player-2,1|from:status:Burn|health:74/270",
            "damage|mon:Toucannon,player-2,1|from:status:Burn|health:28/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
