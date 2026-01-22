//! Tests for the move Echoed Voice.

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
                        "Echoed Voice",
                        "Recover",
                        "Fly",
                        "Tackle"
                    ],
                    "nature": "Modest",
                    "gender": "M",
                    "level": 50,
                    "ivs": {
                        "hp": 31,
                        "atk": 0,
                        "def": 31,
                        "spa": 31,
                        "spd": 31,
                        "spe": 31
                    },
                    "evs": {
                        "hp": 252,
                        "def": 252
                    }
                },
                {
                    "name": "Whimsicott",
                    "species": "Whimsicott",
                    "ability": "No Ability",
                    "moves": [
                        "Echoed Voice",
                        "Tackle"
                    ],
                    "nature": "Timid",
                    "gender": "M",
                    "level": 50,
                    "ivs": {
                        "hp": 31,
                        "atk": 0,
                        "def": 31,
                        "spa": 31,
                        "spd": 31,
                        "spe": 31
                    },
                    "evs": {
                        "hp": 252,
                        "def": 252
                    }
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
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
fn echoed_voice_powers_up_consecutively_and_caps() {
    let mut battle = make_battle(BattleType::Singles, 0, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Echoed Voice (40 BP)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Echoed Voice (80 BP)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Echoed Voice (120 BP)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 4: Echoed Voice (160 BP)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(())); // Use Recover (move index 1)

    // Turn 5: Echoed Voice (200 BP - cap)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:188/210",
            "damage|mon:Audino,player-2,1|health:90/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:148/210",
            "damage|mon:Audino,player-2,1|health:71/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:91/210",
            "damage|mon:Audino,player-2,1|health:44/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:10/210",
            "damage|mon:Audino,player-2,1|health:5/100",
            "move|mon:Audino,player-2,1|name:Recover|target:Audino,player-2,1",
            "split|side:1",
            "heal|mon:Audino,player-2,1|health:115/210",
            "heal|mon:Audino,player-2,1|health:55/100",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:16/210",
            "damage|mon:Audino,player-2,1|health:8/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn echoed_voice_resets_power_if_different_move_is_used() {
    let mut battle = make_battle(BattleType::Singles, 0, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Echoed Voice (40 BP)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Echoed Voice (80 BP)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Tackle (resets power)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3,1"), Ok(())); // Use Tackle (move index 3)
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 4: Echoed Voice (40 BP, power reset)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:188/210",
            "damage|mon:Audino,player-2,1|health:90/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:148/210",
            "damage|mon:Audino,player-2,1|health:71/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Audino,player-1,1|name:Tackle|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:138/210",
            "damage|mon:Audino,player-2,1|health:66/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:117/210",
            "damage|mon:Audino,player-2,1|health:56/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn echoed_voice_powers_up_when_used_by_different_pokemon() {
    let mut battle = make_battle(BattleType::Doubles, 0, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Player 1 Audino uses Echoed Voice (40 BP)
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    // Turn 2: Player 2 Audino uses Echoed Voice (80 BP)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    // Turn 3: Player 1 Whimsicott uses Echoed Voice (120 BP)
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:188/210",
            "damage|mon:Audino,player-2,1|health:90/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Audino,player-2,1|name:Echoed Voice|target:Audino,player-1,1",
            "split|side:0",
            "damage|mon:Audino,player-1,1|health:170/210",
            "damage|mon:Audino,player-1,1|health:81/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Whimsicott,player-1,2|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:146/210",
            "damage|mon:Audino,player-2,1|health:70/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn echoed_voice_miss_resets_power() {
    let mut battle = make_battle(BattleType::Singles, 0, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Player 1 Audino uses Echoed Voice (power 40 BP). Player 2 Audino uses Fly.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(())); // Audino uses Fly (move index 2)

    // Turn 2: Player 1 Audino uses Echoed Voice. It should miss Player 2 Audino. Player 2 Audino
    // uses Fly (attacks).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(())); // Fly is a locked move at index 0.

    // Turn 3: Player 1 Audino uses Echoed Voice. Verify power is reset to 40 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(())); // Player 2 passes

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:188/210",
            "damage|mon:Audino,player-2,1|health:90/100",
            "move|mon:Audino,player-2,1|name:Fly|noanim",
            "prepare|mon:Audino,player-2,1|move:Fly",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Audino,player-1,1|name:Echoed Voice|noanim",
            "miss|mon:Audino,player-2,1",
            "move|mon:Audino,player-2,1|name:Fly|target:Audino,player-1,1",
            "split|side:0",
            "damage|mon:Audino,player-1,1|health:194/210",
            "damage|mon:Audino,player-1,1|health:93/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Audino,player-1,1|name:Echoed Voice|target:Audino,player-2,1",
            "split|side:1",
            "damage|mon:Audino,player-2,1|health:131/210",
            "damage|mon:Audino,player-2,1|health:63/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
