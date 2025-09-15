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

fn male_wobbuffet() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Wobbuffet",
                    "species": "Wobbuffet",
                    "ability": "No Ability",
                    "moves": [
                        "Attract",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gender": "Male"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn female_wobbuffet() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Wobbuffet",
                    "species": "Wobbuffet",
                    "ability": "No Ability",
                    "moves": [
                        "Attract",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gender": "Female"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn female_wobbuffet_with_destiny_knot() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Wobbuffet",
                    "species": "Wobbuffet",
                    "ability": "No Ability",
                    "moves": [
                        "Attract",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gender": "Female",
                    "item": "Destiny Knot"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn unown() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Unown",
                    "species": "Unown",
                    "ability": "No Ability",
                    "moves": [
                        "Attract",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gender": "Unknown"
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
fn attract_causes_infatuation() {
    let mut battle = make_battle(
        0,
        male_wobbuffet().unwrap(),
        female_wobbuffet().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wobbuffet,player-1,1|name:Attract|target:Wobbuffet,player-2,1",
            "start|mon:Wobbuffet,player-2,1|move:Attract",
            "residual",
            "turn|turn:2",
            ["time"],
            "activate|mon:Wobbuffet,player-2,1|move:Attract|of:Wobbuffet,player-1,1",
            "cant|mon:Wobbuffet,player-2,1|from:move:Attract",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn destiny_knot_causes_mutual_attraction() {
    let mut battle = make_battle(
        0,
        male_wobbuffet().unwrap(),
        female_wobbuffet_with_destiny_knot().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wobbuffet,player-1,1|name:Attract|target:Wobbuffet,player-2,1",
            "start|mon:Wobbuffet,player-2,1|move:Attract",
            "start|mon:Wobbuffet,player-1,1|move:Attract|from:item:Destiny Knot|of:Wobbuffet,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn attract_fails_for_equal_genders() {
    let mut battle = make_battle(
        0,
        male_wobbuffet().unwrap(),
        male_wobbuffet().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wobbuffet,player-1,1|name:Attract|noanim",
            "fail|mon:Wobbuffet,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn attract_fails_for_unknown_gender() {
    let mut battle = make_battle(0, unown().unwrap(), male_wobbuffet().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Unown,player-1,1|name:Attract|noanim",
            "fail|mon:Unown,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
