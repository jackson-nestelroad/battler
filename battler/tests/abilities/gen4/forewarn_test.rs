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

fn musharna() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Musharna",
                    "species": "Musharna",
                    "ability": "Forewarn",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Musharna",
                    "species": "Musharna",
                    "ability": "Forewarn",
                    "moves": [],
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
fn forewarn_reveals_strongest_move_of_one_foe() {
    let mut team = musharna().unwrap();
    team.members[0].moves = vec![
        "Tackle".to_owned(),
        "Pound".to_owned(),
        "Dark Pulse".to_owned(),
    ];
    team.members[1].moves = vec!["Tackle".to_owned(), "Pound".to_owned()];
    let mut battle = make_battle(0, musharna().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "activate|mon:Musharna,player-1,2|ability:Forewarn|move:Dark Pulse|of:Musharna,player-2,1",
            "activate|mon:Musharna,player-1,1|ability:Forewarn|move:Dark Pulse|of:Musharna,player-2,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn forewarn_reveals_strongest_move_of_one_foe_using_rng_for_ties() {
    let mut team = musharna().unwrap();
    team.members[0].moves = vec!["Tackle".to_owned(), "Pound".to_owned()];
    team.members[1].moves = vec!["Tackle".to_owned(), "Pound".to_owned()];
    let mut battle = make_battle(
        837467192384912,
        musharna().unwrap(),
        team,
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "activate|mon:Musharna,player-1,2|ability:Forewarn|move:Pound|of:Musharna,player-2,2",
            "activate|mon:Musharna,player-1,1|ability:Forewarn|move:Tackle|of:Musharna,player-2,2",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn forewarn_reveals_ohko_move() {
    let mut team = musharna().unwrap();
    team.members[0].moves = vec!["Dark Pulse".to_owned(), "Guillotine".to_owned()];
    let mut battle = make_battle(0, musharna().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "activate|mon:Musharna,player-1,2|ability:Forewarn|move:Guillotine|of:Musharna,player-2,1",
            "activate|mon:Musharna,player-1,1|ability:Forewarn|move:Guillotine|of:Musharna,player-2,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn forewarn_reveals_attacking_move_with_no_base_power() {
    let mut team = musharna().unwrap();
    team.members[0].moves = vec!["Tackle".to_owned(), "Magnitude".to_owned()];
    let mut battle = make_battle(0, musharna().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "activate|mon:Musharna,player-1,2|ability:Forewarn|move:Magnitude|of:Musharna,player-2,1",
            "activate|mon:Musharna,player-1,1|ability:Forewarn|move:Magnitude|of:Musharna,player-2,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn forewarn_reveals_counter() {
    let mut team = musharna().unwrap();
    team.members[0].moves = vec!["Counter".to_owned(), "Earthquake".to_owned()];
    let mut battle = make_battle(0, musharna().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "activate|mon:Musharna,player-1,2|ability:Forewarn|move:Counter|of:Musharna,player-2,1",
            "activate|mon:Musharna,player-1,1|ability:Forewarn|move:Counter|of:Musharna,player-2,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
