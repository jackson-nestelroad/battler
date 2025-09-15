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
    assert_logs_since_start_eq,
    static_local_data_store,
};

fn toxicroak() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Toxicroak",
                    "species": "Toxicroak",
                    "ability": "Anticipation",
                    "moves": [
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
fn anticipation_does_not_activate_with_no_super_effective_moves() {
    let mut battle = make_battle(
        0,
        toxicroak().unwrap(),
        toxicroak().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn anticipation_does_not_activate_has_super_effective_status_move() {
    let mut team = toxicroak().unwrap();
    team.members[0].moves = vec!["Hypnosis".to_owned()];
    let mut battle = make_battle(0, toxicroak().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn anticipation_activates_when_target_has_super_effective_moves() {
    let mut team = toxicroak().unwrap();
    team.members[0].moves = vec!["Psychic".to_owned()];
    let mut battle = make_battle(0, toxicroak().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "ability|mon:Toxicroak,player-1,1|ability:Anticipation",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn anticipation_activates_when_target_has_ohko_move() {
    let mut team = toxicroak().unwrap();
    team.members[0].moves = vec!["Horn Drill".to_owned()];
    let mut battle = make_battle(0, toxicroak().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "ability|mon:Toxicroak,player-1,1|ability:Anticipation",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
