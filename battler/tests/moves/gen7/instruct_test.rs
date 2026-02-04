use anyhow::Result;
use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
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
use serde_json;

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Oranguru",
                    "species": "Oranguru",
                    "ability": "No Ability",
                    "moves": [
                        "Instruct"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Oranguru",
                    "species": "Oranguru",
                    "ability": "No Ability",
                    "moves": [
                        "Swords Dance",
                        "Tackle",
                        "Focus Punch",
                        "Skull Bash",
                        "Bide"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn instruct_fails_if_no_last_move() {
    let mut battle = make_battle(0, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oranguru,player-1,1|name:Instruct|noanim",
            "fail|mon:Oranguru,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn instruct_forces_target_to_use_last_move() {
    let mut battle = make_battle(0, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;move 1,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oranguru,player-1,2|name:Swords Dance|target:Oranguru,player-1,2",
            "boost|mon:Oranguru,player-1,2|stat:atk|by:2",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Oranguru,player-1,1|name:Instruct|target:Oranguru,player-1,2",
            "singleturn|mon:Oranguru,player-1,2|move:Instruct|of:Oranguru,player-1,1",
            "move|mon:Oranguru,player-1,2|name:Swords Dance|target:Oranguru,player-1,2|from:move:Instruct",
            "boost|mon:Oranguru,player-1,2|stat:atk|by:2",
            "move|mon:Oranguru,player-1,2|name:Tackle|target:Oranguru,player-2,1",
            "split|side:1",
            "damage|mon:Oranguru,player-2,1|health:87/150",
            "damage|mon:Oranguru,player-2,1|health:58/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn instruct_fails_during_focus_punch() {
    let mut battle = make_battle(0, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;move 2,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oranguru,player-1,2|name:Swords Dance|target:Oranguru,player-1,2",
            "boost|mon:Oranguru,player-1,2|stat:atk|by:2",
            "residual",
            "turn|turn:2",
            "continue",
            "singleturn|mon:Oranguru,player-1,2|move:Focus Punch",
            "move|mon:Oranguru,player-1,1|name:Instruct|noanim",
            "fail|mon:Oranguru,player-1,1",
            "move|mon:Oranguru,player-1,2|name:Focus Punch|target:Oranguru,player-2,1",
            "split|side:1",
            "damage|mon:Oranguru,player-2,1|health:48/150",
            "damage|mon:Oranguru,player-2,1|health:32/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn instruct_fails_if_last_move_was_two_turn() {
    let mut battle = make_battle(0, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 3,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oranguru,player-1,2|name:Skull Bash|noanim",
            "prepare|mon:Oranguru,player-1,2|move:Skull Bash",
            "boost|mon:Oranguru,player-1,2|stat:def|by:1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Oranguru,player-1,1|name:Instruct|noanim",
            "fail|mon:Oranguru,player-1,1",
            "move|mon:Oranguru,player-1,2|name:Skull Bash|target:Oranguru,player-2,1",
            "split|side:1",
            "damage|mon:Oranguru,player-2,1|health:83/150",
            "damage|mon:Oranguru,player-2,1|health:56/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Oranguru,player-1,1|name:Instruct|noanim",
            "fail|mon:Oranguru,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn instruct_fails_if_last_move_was_bide() {
    let mut battle = make_battle(0, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 4"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 1,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oranguru,player-1,2|name:Bide|target:Oranguru,player-1,2",
            "start|mon:Oranguru,player-1,2|move:Bide",
            "move|mon:Oranguru,player-2,2|name:Tackle|target:Oranguru,player-1,2",
            "split|side:0",
            "damage|mon:Oranguru,player-1,2|health:128/150",
            "damage|mon:Oranguru,player-1,2|health:86/100",
            "residual",
            "turn|turn:2",
            "continue",
            "activate|mon:Oranguru,player-1,2|move:Bide",
            "move|mon:Oranguru,player-1,2|name:Bide|target:Oranguru,player-1,2",
            "move|mon:Oranguru,player-1,1|name:Instruct|noanim",
            "fail|mon:Oranguru,player-1,1",
            "residual",
            "turn|turn:3",
            "continue",
            "end|mon:Oranguru,player-1,2|move:Bide",
            "move|mon:Oranguru,player-1,2|name:Bide|target:Oranguru,player-2,2",
            "split|side:1",
            "damage|mon:Oranguru,player-2,2|health:106/150",
            "damage|mon:Oranguru,player-2,2|health:71/100",
            "move|mon:Oranguru,player-1,1|name:Instruct|noanim",
            "fail|mon:Oranguru,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
