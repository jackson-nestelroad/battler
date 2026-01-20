//! Tests for the move Clear Smog.
//!
//! # Test Cases
//!
//! - Clear Smog resets positive boosts.
//! - Clear Smog resets negative boosts.
//! - Clear Smog is blocked by move immunity (e.g., Steel-types).
//! - Clear Smog is blocked by Protect.
//! - Clear Smog is blocked by Substitute.

use anyhow::Result;
use battler::{
    battle::{
        BattleType,
        PublicCoreBattle,
    },
    teams::TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn amoonguss_team() -> TeamData {
    serde_json::from_str(
        r#"{
        "members": [
            {
                "species": "Amoonguss",
                "name": "Amoonguss",
                "ability": "No Ability",
                "moves": [
                    "Clear Smog",
                    "Nasty Plot",
                    "Charm",
                    "Protect",
                    "Substitute"
                ]
            }
        ]
    }"#,
    )
    .unwrap()
}

fn make_battle(team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_seed(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn clear_smog_resets_positive_boosts() {
    let mut battle = make_battle(amoonguss_team(), amoonguss_team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
        "move|mon:Amoonguss,player-2,1|name:Nasty Plot|target:Amoonguss,player-2,1",
        "boost|mon:Amoonguss,player-2,1|stat:spa|by:2",
        "residual",
        "turn|turn:2",
        "continue",
        "move|mon:Amoonguss,player-1,1|name:Clear Smog|target:Amoonguss,player-2,1",
        "split|side:1",
        [
            "damage",
            "mon:Amoonguss,player-2,1"
        ],
        [
            "damage",
            "mon:Amoonguss,player-2,1"
        ],
        "clearboosts|mon:Amoonguss,player-2,1|from:move:Clear Smog|of:Amoonguss,player-1,1",
        "residual",
        "turn|turn:3"
    ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn clear_smog_resets_negative_boosts() {
    let mut battle = make_battle(amoonguss_team(), amoonguss_team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
        "move|mon:Amoonguss,player-1,1|name:Charm|target:Amoonguss,player-2,1",
        "unboost|mon:Amoonguss,player-2,1|stat:atk|by:2",
        "residual",
        "turn|turn:2",
        "continue",
        "move|mon:Amoonguss,player-1,1|name:Clear Smog|target:Amoonguss,player-2,1",
        "split|side:1",
        [
            "damage",
            "mon:Amoonguss,player-2,1"
        ],
        [
            "damage",
            "mon:Amoonguss,player-2,1"
        ],
        "clearboosts|mon:Amoonguss,player-2,1|from:move:Clear Smog|of:Amoonguss,player-1,1",
        "residual",
        "turn|turn:3"
    ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

fn steel_team() -> TeamData {
    serde_json::from_str(
        r#"{
        "members": [
            {
                "species": "Klinklang",
            "name": "Klinklang",
            "ability": "No Ability",
                "moves": ["Shift Gear"]
            }
        ]
    }"#,
    )
    .unwrap()
}

#[test]
fn clear_smog_is_blocked_by_immunity() {
    let mut battle = make_battle(amoonguss_team(), steel_team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
        "move|mon:Klinklang,player-2,1|name:Shift Gear|target:Klinklang,player-2,1",
        "boost|mon:Klinklang,player-2,1|stat:atk|by:2",
        "boost|mon:Klinklang,player-2,1|stat:spe|by:2",
        "residual",
        "turn|turn:2",
        "continue",
        "move|mon:Amoonguss,player-1,1|name:Clear Smog|noanim",
        "immune|mon:Klinklang,player-2,1",
        "residual",
        "turn|turn:3"
    ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn clear_smog_is_blocked_by_protect() {
    let mut battle = make_battle(amoonguss_team(), amoonguss_team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
        "move|mon:Amoonguss,player-2,1|name:Nasty Plot|target:Amoonguss,player-2,1",
        "boost|mon:Amoonguss,player-2,1|stat:spa|by:2",
        "residual",
        "turn|turn:2",
        "continue",
        "move|mon:Amoonguss,player-2,1|name:Protect|target:Amoonguss,player-2,1",
        "singleturn|mon:Amoonguss,player-2,1|move:Protect",
        "move|mon:Amoonguss,player-1,1|name:Clear Smog|target:Amoonguss,player-2,1",
        "activate|mon:Amoonguss,player-2,1|move:Protect",
        "residual",
        "turn|turn:3"
    ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
#[ignore]
fn clear_smog_is_blocked_by_substitute() {
    let mut battle = make_battle(amoonguss_team(), amoonguss_team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
        "move|mon:Amoonguss,player-2,1|name:Nasty Plot",
        "boost|mon:Amoonguss,player-2,1|stat:spa|amount:2",
        "residual",
        "turn|turn:2",
        "move|mon:Amoonguss,player-2,1|name:Substitute",
        "split|side:1",
        "start|mon:Amoonguss,player-2,1|effect:Substitute",
        "move|mon:Amoonguss,player-1,1|name:Clear Smog|target:Amoonguss,player-2,1",
        "activate|mon:Amoonguss,player-2,1|effect:Substitute|of:Amoonguss,player-1,1",
        "damage|mon:Amoonguss,player-2,1|from:substitute",
        "residual",
        "turn|turn:3"
    ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
