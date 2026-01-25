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

fn budew() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Budew",
                    "species": "Budew",
                    "ability": "Leaf Guard",
                    "moves": [
                        "Sunny Day",
                        "Thunder Wave",
                        "Yawn",
                        "Skill Swap"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn leaf_guard_prevents_status_in_sun() {
    let mut battle = make_battle(0, budew().unwrap(), budew().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Budew,player-1,1|name:Thunder Wave|noanim",
            "immune|mon:Budew,player-2,1|from:ability:Leaf Guard",
            "fail|mon:Budew,player-1,1",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn leaf_guard_prevents_yawn_in_sun() {
    let mut battle = make_battle(0, budew().unwrap(), budew().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Budew,player-1,1|name:Yawn|noanim",
            "activate|mon:Budew,player-2,1|ability:Leaf Guard",
            "fail|mon:Budew,player-1,1",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn leaf_guard_prevents_previous_yawn_in_sun() {
    let mut team = budew().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, budew().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Budew,player-1,1|name:Yawn|target:Budew,player-2,1",
            "start|mon:Budew,player-2,1|move:Yawn|of:Budew,player-1,1",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Budew,player-1,1|name:Skill Swap|target:Budew,player-2,1",
            "activate|mon:Budew,player-2,1|move:Skill Swap|of:Budew,player-1,1",
            "abilityend|mon:Budew,player-1,1|ability:Leaf Guard|from:move:Skill Swap",
            "ability|mon:Budew,player-1,1|ability:No Ability|from:move:Skill Swap",
            "abilityend|mon:Budew,player-2,1|ability:No Ability|from:move:Skill Swap|of:Budew,player-1,1",
            "ability|mon:Budew,player-2,1|ability:Leaf Guard|from:move:Skill Swap|of:Budew,player-1,1",
            "weather|weather:Harsh Sunlight|residual",
            "end|mon:Budew,player-2,1|move:Yawn|silent",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
