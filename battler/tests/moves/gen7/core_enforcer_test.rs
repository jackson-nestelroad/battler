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
                    "name": "Zygarde",
                    "species": "Zygarde",
                    "ability": "Levitate",
                    "moves": [
                        "Core Enforcer",
                        "Earthquake"
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
fn core_enforcer_suppresses_ability_if_target_moved() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Zygarde,player-1,1|name:Core Enforcer",
            "supereffective|mon:Zygarde,player-2,1",
            "split|side:1",
            "damage|mon:Zygarde,player-2,1|health:116/326",
            "damage|mon:Zygarde,player-2,1|health:36/100",
            "move|mon:Zygarde,player-2,1|name:Core Enforcer",
            "supereffective|mon:Zygarde,player-1,1",
            "split|side:0",
            "damage|mon:Zygarde,player-1,1|health:132/326",
            "damage|mon:Zygarde,player-1,1|health:41/100",
            "abilityend|mon:Zygarde,player-1,1|ability:Levitate|from:move:Gastro Acid|of:Zygarde,player-2,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Zygarde,player-1,1|name:Earthquake|noanim",
            "immune|mon:Zygarde,player-2,1",
            "move|mon:Zygarde,player-2,1|name:Earthquake",
            "split|side:0",
            "damage|mon:Zygarde,player-1,1|health:42/326",
            "damage|mon:Zygarde,player-1,1|health:13/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
