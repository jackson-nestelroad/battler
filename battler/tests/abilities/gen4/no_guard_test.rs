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

fn machamp() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Machamp",
                    "species": "Machamp",
                    "ability": "No Guard",
                    "moves": [
                        "Guillotine",
                        "Fly",
                        "Tackle"
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
fn no_guard_makes_move_from_target_accuracy_exempt() {
    let mut team = machamp().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(100, machamp().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Machamp,player-1,1|name:Guillotine|target:Machamp,player-2,1",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:0",
            "damage|mon:Machamp,player-2,1|health:0",
            "ohko|mon:Machamp,player-2,1",
            "faint|mon:Machamp,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn no_guard_makes_move_against_target_accuracy_exempt() {
    let mut team = machamp().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(100, machamp().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Machamp,player-2,1|name:Guillotine|target:Machamp,player-1,1",
            "split|side:0",
            "damage|mon:Machamp,player-1,1|health:0",
            "damage|mon:Machamp,player-1,1|health:0",
            "ohko|mon:Machamp,player-1,1",
            "faint|mon:Machamp,player-1,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn no_guard_makes_move_hit_from_target_through_semi_invulnerability() {
    let mut team = machamp().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(100, machamp().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Machamp,player-2,1|name:Fly|noanim",
            "prepare|mon:Machamp,player-2,1|move:Fly",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Machamp,player-1,1|name:Tackle|target:Machamp,player-2,1",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:126/150",
            "damage|mon:Machamp,player-2,1|health:84/100",
            "move|mon:Machamp,player-2,1|name:Fly|target:Machamp,player-1,1",
            "supereffective|mon:Machamp,player-1,1",
            "split|side:0",
            "damage|mon:Machamp,player-1,1|health:40/150",
            "damage|mon:Machamp,player-1,1|health:27/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn no_guard_makes_move_hit_against_target_through_semi_invulnerability() {
    let mut team = machamp().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(100, machamp().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Machamp,player-1,1|name:Fly|noanim",
            "prepare|mon:Machamp,player-1,1|move:Fly",
            "move|mon:Machamp,player-2,1|name:Tackle|target:Machamp,player-1,1",
            "split|side:0",
            "damage|mon:Machamp,player-1,1|health:126/150",
            "damage|mon:Machamp,player-1,1|health:84/100",
            "residual",
            "turn|turn:2"  
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
