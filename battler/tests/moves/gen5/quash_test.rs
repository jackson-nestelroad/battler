use anyhow::Result;
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
use serde_json;

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Whimsicott",
                    "species": "Whimsicott",
                    "ability": "Prankster",
                    "moves": [
                        "Quash",
                        "Quick Attack",
                        "Scratch"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Slowpoke",
                    "species": "Slowpoke",
                    "ability": "No Ability",
                    "moves": [
                        "Scratch"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn quash_deprioritizes_target() {
    let mut battle = make_battle(0, team(), team()).unwrap();

    // Whimsicott (Fast) uses Quash on Opponent Whimsicott.
    // Opponent Whimsicott (Fast) uses Scratch.
    // Quashed target moves last.

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0,2"),
        Ok(())
    ); // Quash targeting P2 L1
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,1;move 0,1"),
        Ok(())
    ); // Scratch targeting P1 L1

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Whimsicott,player-1,1|name:Quash|target:Whimsicott,player-2,1",
            "activate|mon:Whimsicott,player-2,1|move:Quash",
            "move|mon:Slowpoke,player-1,2|name:Scratch|target:Slowpoke,player-2,2",
            "split|side:1",
            "damage|mon:Slowpoke,player-2,2|health:132/150",
            "damage|mon:Slowpoke,player-2,2|health:88/100",
            "move|mon:Slowpoke,player-2,2|name:Scratch|target:Whimsicott,player-1,1",
            "split|side:0",
            "damage|mon:Whimsicott,player-1,1|health:107/120",
            "damage|mon:Whimsicott,player-1,1|health:90/100",
            "move|mon:Whimsicott,player-2,1|name:Scratch|target:Whimsicott,player-1,1",
            "split|side:0",
            "damage|mon:Whimsicott,player-1,1|health:92/120",
            "damage|mon:Whimsicott,player-1,1|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn quash_fails_if_target_moved() {
    let mut battle = make_battle(0, team(), team()).unwrap();

    // P1 Whimsicott uses Quick Attack (Priority +1).
    // P2 Whimsicott uses Quash (Prankster +1) on P1 Whimsicott.
    // P1 moves first due to speed tie/order.
    // Quash fails because target already moved.

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;move 0,1"),
        Ok(())
    ); // Quick Attack
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 0,1"),
        Ok(())
    ); // Quash targeting P1 L1

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Whimsicott,player-1,1|name:Quick Attack|target:Whimsicott,player-2,1",
            "split|side:1",
            "damage|mon:Whimsicott,player-2,1|health:105/120",
            "damage|mon:Whimsicott,player-2,1|health:88/100",
            "move|mon:Whimsicott,player-2,1|name:Quash|noanim",
            "fail|mon:Whimsicott,player-2,1",
            "move|mon:Slowpoke,player-1,2|name:Scratch|target:Whimsicott,player-2,1",
            "split|side:1",
            "damage|mon:Whimsicott,player-2,1|health:92/120",
            "damage|mon:Whimsicott,player-2,1|health:77/100",
            "move|mon:Slowpoke,player-2,2|name:Scratch|target:Whimsicott,player-1,1",
            "split|side:0",
            "damage|mon:Whimsicott,player-1,1|health:106/120",
            "damage|mon:Whimsicott,player-1,1|health:89/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn quash_ignores_priority() {
    let mut battle = make_battle(0, team(), team()).unwrap();

    // P1 Whimsicott uses Quash (Prankster +1) on P2 Whimsicott.
    // P2 Whimsicott uses Quick Attack (Priority +1).
    // Quash forces P2 Whimsicott to move last, ignoring priority.

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0,2"),
        Ok(())
    ); // Quash targeting P2 L1
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 0,1"),
        Ok(())
    ); // Quick Attack targeting P1 L1

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Whimsicott,player-1,1|name:Quash|target:Whimsicott,player-2,1",
            "activate|mon:Whimsicott,player-2,1|move:Quash",
            "move|mon:Slowpoke,player-1,2|name:Scratch|target:Slowpoke,player-2,2",
            "split|side:1",
            "damage|mon:Slowpoke,player-2,2|health:132/150",
            "damage|mon:Slowpoke,player-2,2|health:88/100",
            "move|mon:Slowpoke,player-2,2|name:Scratch|target:Whimsicott,player-1,1",
            "split|side:0",
            "damage|mon:Whimsicott,player-1,1|health:107/120",
            "damage|mon:Whimsicott,player-1,1|health:90/100",
            "move|mon:Whimsicott,player-2,1|name:Quick Attack|target:Whimsicott,player-1,1",
            "split|side:0",
            "damage|mon:Whimsicott,player-1,1|health:92/120",
            "damage|mon:Whimsicott,player-1,1|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
