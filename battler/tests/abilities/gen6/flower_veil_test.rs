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
                    "name": "Floette",
                    "species": "Floette",
                    "ability": "Flower Veil",
                    "moves": [
                        "Snarl",
                        "Yawn",
                        "Thunder Wave",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Chespin",
                    "species": "Chespin",
                    "ability": "No Ability",
                    "moves": [
                        "Rest"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Weezing",
                    "species": "Weezing",
                    "ability": "Neutralizing Gas",
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
fn flower_veil_protects_grass_type_ally() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Floette,player-2,1|name:Snarl|spread:Floette,player-1,1;Chespin,player-1,2",
            "resisted|mon:Floette,player-1,1",
            "split|side:0",
            "damage|mon:Floette,player-1,1|health:205/218",
            "damage|mon:Floette,player-1,1|health:95/100",
            "split|side:0",
            "damage|mon:Chespin,player-1,2|health:170/222",
            "damage|mon:Chespin,player-1,2|health:77/100",
            "unboost|mon:Floette,player-1,1|stat:spa|by:1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Floette,player-2,1|name:Yawn|noanim",
            "block|mon:Chespin,player-1,2|move:Yawn|from:ability:Flower Veil",
            "fail|mon:Floette,player-2,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Floette,player-2,1|name:Thunder Wave|noanim",
            "block|mon:Chespin,player-1,2|move:Thunder Wave|from:ability:Flower Veil",
            "fail|mon:Floette,player-2,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_veil_does_not_protect_against_rest() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Floette,player-1,1|name:Tackle|target:Chespin,player-2,2",
            "split|side:1",
            "damage|mon:Chespin,player-2,2|health:198/222",
            "damage|mon:Chespin,player-2,2|health:90/100",
            "move|mon:Chespin,player-2,2|name:Rest|target:Chespin,player-2,2",
            "status|mon:Chespin,player-2,2|status:Sleep",
            "split|side:1",
            "heal|mon:Chespin,player-2,2|health:222/222",
            "heal|mon:Chespin,player-2,2|health:100/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flower_veil_does_not_protect_against_existing_rest() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 0;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Weezing"],
            ["switch", "player-1", "Weezing"],
            "ability|mon:Weezing,player-1,1|ability:Neutralizing Gas",
            "move|mon:Floette,player-2,1|name:Yawn|target:Chespin,player-1,2",
            "start|mon:Chespin,player-1,2|move:Yawn|of:Floette,player-2,1",
            "residual",
            "turn|turn:2",
            "continue",
            "end|mon:Weezing,player-1,1|ability:Neutralizing Gas",
            "split|side:0",
            ["switch", "player-1", "Floette"],
            ["switch", "player-1", "Floette"],
            "move|mon:Floette,player-2,1|name:Thunder Wave|noanim",
            "block|mon:Chespin,player-1,2|move:Thunder Wave|from:ability:Flower Veil",
            "fail|mon:Floette,player-2,1",
            "end|mon:Chespin,player-1,2|move:Yawn|silent",
            "status|mon:Chespin,player-1,2|status:Sleep|from:move:Yawn",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
