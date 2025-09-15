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

fn talonflame() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Talonflame",
                    "species": "Talonflame",
                    "ability": "No Ability",
                    "item": "Iron Ball",
                    "moves": [
                        "Earthquake",
                        "Gravity"
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
fn iron_ball_grounds_holder_and_makes_ground_types_normal_effective() {
    let mut battle = make_battle(0, talonflame().unwrap(), talonflame().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Talonflame,player-2,1|name:Earthquake",
            "split|side:0",
            "damage|mon:Talonflame,player-1,1|health:89/138",
            "damage|mon:Talonflame,player-1,1|health:65/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn iron_ball_does_not_modify_effectiveness_if_holder_is_grounded_by_other_effect() {
    let mut battle = make_battle(0, talonflame().unwrap(), talonflame().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Talonflame,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "move|mon:Talonflame,player-2,1|name:Earthquake",
            "supereffective|mon:Talonflame,player-1,1",
            "split|side:0",
            "damage|mon:Talonflame,player-1,1|health:40/138",
            "damage|mon:Talonflame,player-1,1|health:29/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
