use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn leavanny() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Leavanny",
                    "species": "Leavanny",
                    "ability": "Swarm",
                    "moves": [
                        "Entrainment"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>, anyhow::Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .with_pass_allowed(true)
        .build(static_local_data_store())
}

#[test]
fn entrainment_changes_ability() {
    let team_1 = leavanny();
    let mut team_2 = leavanny();
    team_2.members[0].ability = "Inner Focus".to_owned();

    let mut battle = make_battle(0, team_1, team_2).unwrap();
    battle.start().unwrap();
    battle.set_player_choice("player-1", "move 0").unwrap();
    battle.set_player_choice("player-2", "pass").unwrap();

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Leavanny,player-1,1|name:Entrainment|target:Leavanny,player-2,1",
            "abilityend|mon:Leavanny,player-2,1|ability:Inner Focus|from:move:Entrainment|of:Leavanny,player-1,1",
            "ability|mon:Leavanny,player-2,1|ability:Swarm|from:move:Entrainment|of:Leavanny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn entrainment_fails_if_same_ability() {
    let team_1 = leavanny();
    let team_2 = leavanny();

    let mut battle = make_battle(0, team_1, team_2).unwrap();
    battle.start().unwrap();
    battle.set_player_choice("player-1", "move 0").unwrap();
    battle.set_player_choice("player-2", "pass").unwrap();

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Leavanny,player-1,1|name:Entrainment|noanim",
            "fail|mon:Leavanny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn entrainment_fails_if_target_is_truant() {
    let team_1 = leavanny();
    let mut team_2 = leavanny();
    team_2.members[0].ability = "Truant".to_owned();

    let mut battle = make_battle(0, team_1, team_2).unwrap();
    battle.start().unwrap();
    battle.set_player_choice("player-1", "move 0").unwrap();
    battle.set_player_choice("player-2", "pass").unwrap();

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Leavanny,player-1,1|name:Entrainment|noanim",
            "fail|mon:Leavanny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn entrainment_fails_if_user_is_trace() {
    let mut team_1 = leavanny();
    team_1.members[0].ability = "Trace".to_owned();
    let mut team_2 = leavanny();
    team_2.members[0].ability = "Inner Focus".to_owned();

    let mut battle = make_battle(0, team_1, team_2).unwrap();
    battle.start().unwrap();
    battle.set_player_choice("player-1", "move 0").unwrap();
    battle.set_player_choice("player-2", "pass").unwrap();

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Leavanny,player-1,1|name:Entrainment|noanim",
            "fail|mon:Leavanny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn entrainment_fails_if_target_is_multitype() {
    let team_1 = leavanny();
    let mut team_2 = leavanny();
    team_2.members[0].ability = "Multitype".to_owned();

    let mut battle = make_battle(0, team_1, team_2).unwrap();
    battle.start().unwrap();
    battle.set_player_choice("player-1", "move 0").unwrap();
    battle.set_player_choice("player-2", "pass").unwrap();

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Leavanny,player-1,1|name:Entrainment|noanim",
            "fail|mon:Leavanny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn entrainment_works_on_wonder_guard() {
    let team_1 = leavanny();
    let mut team_2 = leavanny();
    team_2.members[0].ability = "Wonder Guard".to_owned();

    let mut battle = make_battle(0, team_1, team_2).unwrap();
    battle.start().unwrap();
    battle.set_player_choice("player-1", "move 0").unwrap();
    battle.set_player_choice("player-2", "pass").unwrap();

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Leavanny,player-1,1|name:Entrainment|target:Leavanny,player-2,1",
            "abilityend|mon:Leavanny,player-2,1|ability:Wonder Guard|from:move:Entrainment|of:Leavanny,player-1,1",
            "ability|mon:Leavanny,player-2,1|ability:Swarm|from:move:Entrainment|of:Leavanny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
