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
                    "name": "Turtonator",
                    "species": "Turtonator",
                    "ability": "No Ability",
                    "moves": [
                        "Shell Trap",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Turtonator",
                    "species": "Turtonator",
                    "ability": "No Ability",
                    "moves": [
                        "Shell Trap",
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
fn shell_trap_activates_on_physical_move() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 1,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Turtonator,player-1,1|move:Shell Trap",
            "singleturn|mon:Turtonator,player-1,2|move:Shell Trap",
            "move|mon:Turtonator,player-2,1|name:Tackle|target:Turtonator,player-1,1",
            "split|side:0",
            "damage|mon:Turtonator,player-1,1|health:210/230",
            "damage|mon:Turtonator,player-1,1|health:92/100",
            "move|mon:Turtonator,player-1,1|name:Shell Trap|spread:Turtonator,player-2,1;Turtonator,player-2,2",
            "resisted|mon:Turtonator,player-2,1",
            "resisted|mon:Turtonator,player-2,2",
            "split|side:1",
            "damage|mon:Turtonator,player-2,1|health:196/230",
            "damage|mon:Turtonator,player-2,1|health:86/100",
            "split|side:1",
            "damage|mon:Turtonator,player-2,2|health:198/230",
            "damage|mon:Turtonator,player-2,2|health:87/100",
            "move|mon:Turtonator,player-2,2|name:Tackle|target:Turtonator,player-1,1",
            "split|side:0",
            "damage|mon:Turtonator,player-1,1|health:191/230",
            "damage|mon:Turtonator,player-1,1|health:84/100",
            "cant|mon:Turtonator,player-1,2|from:move:Shell Trap",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
