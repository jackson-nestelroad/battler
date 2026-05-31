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
                    "name": "Slowbro",
                    "species": "Slowbro-Galar",
                    "ability": "No Ability",
                    "moves": [
                        "Shell Side Arm"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Regice",
                    "species": "Regice",
                    "ability": "No Ability",
                    "moves": [],
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
        .with_battle_type(BattleType::Doubles)
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
fn shell_side_arm_can_be_physical_or_special() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Slowbro,player-1,1|name:Shell Side Arm|target:Slowbro,player-2,1|anim:Special",
            "resisted|mon:Slowbro,player-2,1",
            "split|side:1",
            "damage|mon:Slowbro,player-2,1|health:222/300",
            "damage|mon:Slowbro,player-2,1|health:74/100",
            "move|mon:Slowbro,player-2,1|name:Shell Side Arm|target:Regice,player-1,2|anim:Physical",
            "split|side:0",
            "damage|mon:Regice,player-1,2|health:165/270",
            "damage|mon:Regice,player-1,2|health:62/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn shell_side_arm_does_not_reveal_category_when_hitting_ally() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Slowbro,player-1,1|name:Shell Side Arm|target:Regice,player-1,2",
            "split|side:0",
            "damage|mon:Regice,player-1,2|health:159/270",
            "damage|mon:Regice,player-1,2|health:59/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
