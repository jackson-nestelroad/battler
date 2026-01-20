//! Tests for the move Round.

use anyhow::Result;
use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
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

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Audino",
                    "species": "Audino",
                    "ability": "No Ability",
                    "moves": [
                        "Round",
                        "Tackle"
                    ],
                    "nature": "Timid",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Audino",
                    "species": "Audino",
                    "ability": "No Ability",
                    "moves": [
                        "Round",
                        "Tackle"
                    ],
                    "nature": "Timid",
                    "gender": "M",
                    "level": 1
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(0)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_pass_allowed(true)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn round_powers_up_and_moves_ally_immediately() {
    let mut battle = make_battle(BattleType::Doubles, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 1,2"),
        Ok(())
    );

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Audino,player-1,1|name:Round|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:135/163",
            "damage|mon:Audino,player-2,1|health:83/100",
            "move|mon:Audino,player-1,2|name:Round|target:Audino,player-2,2|from:move:Round",
            "split|side:1",
            "damage|mon:Audino,player-2,2|health:6/13",
            "damage|mon:Audino,player-2,2|health:47/100",
            "move|mon:Audino,player-2,1|name:Tackle|target:Audino,player-1,1",
            "split|side:0",
            "damage|mon:Audino,player-1,1|health:147/163",
            "damage|mon:Audino,player-1,1|health:91/100",
            "move|mon:Audino,player-2,2|name:Tackle|target:Audino,player-1,2",
            "split|side:0",
            "damage|mon:Audino,player-1,2|health:10/13",
            "damage|mon:Audino,player-1,2|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn round_does_not_power_up_if_ally_does_not_use_round() {
    let mut battle = make_battle(BattleType::Doubles, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 1,2"),
        Ok(())
    );

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Audino,player-1,1|name:Tackle|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:145/163",
            "damage|mon:Audino,player-2,1|health:89/100",
            "move|mon:Audino,player-2,1|name:Tackle|target:Audino,player-1,1",
            "split|side:0",
            "damage|mon:Audino,player-1,1|health:147/163",
            "damage|mon:Audino,player-1,1|health:91/100",
            "move|mon:Audino,player-1,2|name:Round|target:Audino,player-2,2",
            "split|side:1",
            "damage|mon:Audino,player-2,2|health:9/13",
            "damage|mon:Audino,player-2,2|health:70/100",
            "move|mon:Audino,player-2,2|name:Tackle|target:Audino,player-1,2",
            "split|side:0",
            "damage|mon:Audino,player-1,2|health:10/13",
            "damage|mon:Audino,player-1,2|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn round_chains_in_doubles() {
    let mut battle = make_battle(BattleType::Doubles, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 0,2"),
        Ok(())
    );

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Audino,player-1,1|name:Round|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:135/163",
            "damage|mon:Audino,player-2,1|health:83/100",
            "move|mon:Audino,player-2,1|name:Round|target:Audino,player-1,1|from:move:Round",
            "split|side:0",
            "damage|mon:Audino,player-1,1|health:111/163",
            "damage|mon:Audino,player-1,1|health:69/100",
            "move|mon:Audino,player-1,2|name:Round|target:Audino,player-2,2|from:move:Round",
            "split|side:1",
            "damage|mon:Audino,player-2,2|health:6/13",
            "damage|mon:Audino,player-2,2|health:47/100",
            "move|mon:Audino,player-2,2|name:Round|target:Audino,player-1,2|from:move:Round",
            "split|side:0",
            "damage|mon:Audino,player-1,2|health:6/13",
            "damage|mon:Audino,player-1,2|health:47/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn round_powers_up_from_opponent() {
    let mut battle = make_battle(BattleType::Doubles, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 1,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 1,2"),
        Ok(())
    );

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Audino,player-1,1|name:Round|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:135/163",
            "damage|mon:Audino,player-2,1|health:83/100",
            "move|mon:Audino,player-2,1|name:Round|target:Audino,player-1,1|from:move:Round",
            "split|side:0",
            "damage|mon:Audino,player-1,1|health:111/163",
            "damage|mon:Audino,player-1,1|health:69/100",
            "move|mon:Audino,player-1,2|name:Tackle|target:Audino,player-2,2",
            "split|side:1",
            "damage|mon:Audino,player-2,2|health:10/13",
            "damage|mon:Audino,player-2,2|health:77/100",
            "move|mon:Audino,player-2,2|name:Tackle|target:Audino,player-1,2",
            "split|side:0",
            "damage|mon:Audino,player-1,2|health:10/13",
            "damage|mon:Audino,player-1,2|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
