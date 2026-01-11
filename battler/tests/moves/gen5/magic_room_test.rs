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

fn gothitelle() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gothitelle",
                    "species": "Gothitelle",
                    "ability": "No Ability",
                    "moves": [
                        "Magic Room",
                        "Psychic",
                        "Thunderbolt",
                        "Bug Buzz"
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
fn magic_room_suppresses_passive_items() {
    let mut team_1 = gothitelle().unwrap();
    team_1.members[0].item = Some("Leftovers".to_owned());

    let mut battle = make_battle(0, team_1, gothitelle().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    // Magic Room ends, so Leftovers heals.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gothitelle,player-1,1|name:Magic Room",
            "fieldstart|move:Magic Room",
            "move|mon:Gothitelle,player-2,1|name:Bug Buzz|target:Gothitelle,player-1,1",
            "supereffective|mon:Gothitelle,player-1,1",
            "split|side:0",
            "damage|mon:Gothitelle,player-1,1|health:62/130",
            "damage|mon:Gothitelle,player-1,1|health:48/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Gothitelle,player-1,1|name:Magic Room",
            "fieldend|move:Magic Room",
            "split|side:0",
            "heal|mon:Gothitelle,player-1,1|from:item:Leftovers|health:70/130",
            "heal|mon:Gothitelle,player-1,1|from:item:Leftovers|health:54/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn magic_room_suppresses_triggered_items() {
    let mut team_1 = gothitelle().unwrap();
    team_1.members[0].item = Some("Sitrus Berry".to_owned());

    let mut battle = make_battle(0, team_1, gothitelle().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    // Magic Room ends, enabling Sitrus Berry which triggers immediately.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gothitelle,player-1,1|name:Magic Room",
            "fieldstart|move:Magic Room",
            "move|mon:Gothitelle,player-2,1|name:Bug Buzz|target:Gothitelle,player-1,1",
            "supereffective|mon:Gothitelle,player-1,1",
            "split|side:0",
            "damage|mon:Gothitelle,player-1,1|health:62/130",
            "damage|mon:Gothitelle,player-1,1|health:48/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Gothitelle,player-1,1|name:Magic Room",
            "fieldend|move:Magic Room",
            "itemend|mon:Gothitelle,player-1,1|item:Sitrus Berry|eat",
            "split|side:0",
            "heal|mon:Gothitelle,player-1,1|from:item:Sitrus Berry|health:94/130",
            "heal|mon:Gothitelle,player-1,1|from:item:Sitrus Berry|health:73/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn magic_room_suppresses_choice_items() {
    let mut team_1 = gothitelle().unwrap();
    team_1.members[0].item = Some("Choice Scarf".to_owned());

    let mut battle = make_battle(0, team_1, gothitelle().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Magic Room suppresses Choice Scarf, so the lock is ignored.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2"),
        Ok(()),
        "Failed to use Thunderbolt (Move 2) in Magic Room"
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // The choice lock was cleared in Turn 2 because the item was disabled.
    // Re-establish the lock on a different move.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Choice Scarf is active again, so P1 is locked into Thunderbolt.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Gothitelle's Psychic is disabled")
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
}

#[test]
fn magic_room_terminates_after_5_turns() {
    let mut battle = make_battle(0, gothitelle().unwrap(), gothitelle().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    for _ in 0..4 {
        assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    }

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gothitelle,player-1,1|name:Magic Room",
            "fieldstart|move:Magic Room",
            "residual",
            "turn|turn:2",
            "continue",
            "residual",
            "turn|turn:3",
            "continue",
            "residual",
            "turn|turn:4",
            "continue",
            "residual",
            "turn|turn:5",
            "continue",
            "fieldend|move:Magic Room",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn magic_room_removed_by_restart() {
    let mut battle = make_battle(0, gothitelle().unwrap(), gothitelle().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gothitelle,player-1,1|name:Magic Room",
            "fieldstart|move:Magic Room",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Gothitelle,player-1,1|name:Magic Room",
            "fieldend|move:Magic Room",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
