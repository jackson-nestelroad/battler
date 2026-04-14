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
                    "name": "Aegislash",
                    "species": "Aegislash",
                    "ability": "Stance Change",
                    "moves": [
                        "Shadow Claw",
                        "Swords Dance",
                        "King's Shield"
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
fn stance_change_changes_aegislash_forme() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "formechange|mon:Aegislash,player-1,1|species:Aegislash-Blade|from:ability:Stance Change",
            "move|mon:Aegislash,player-1,1|name:Shadow Claw|target:Aegislash,player-2,1",
            "supereffective|mon:Aegislash,player-2,1",
            "split|side:1",
            "damage|mon:Aegislash,player-2,1|health:56/230",
            "damage|mon:Aegislash,player-2,1|health:25/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Aegislash,player-1,1|name:Swords Dance|target:Aegislash,player-1,1",
            "boost|mon:Aegislash,player-1,1|stat:atk|by:2",
            "residual",
            "turn|turn:3",
            "continue",
            "formechange|mon:Aegislash,player-1,1|species:Aegislash|from:ability:Stance Change",
            "move|mon:Aegislash,player-1,1|name:King's Shield|noanim",
            "fail|mon:Aegislash,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
