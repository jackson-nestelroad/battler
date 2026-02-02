use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn oricorio() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Oricorio",
                    "species": "Oricorio-Baile",
                    "ability": "Dancer",
                    "moves": [
                        "Tackle",
                        "Swords Dance",
                        "Fiery Dance",
                        "Protect",
                        "Confuse Ray",
                        "Petal Dance"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Oricorio",
                    "species": "Oricorio-Pom-Pom",
                    "ability": "Dancer",
                    "moves": [
                        "Swords Dance"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    seed: u64,
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn dancer_copies_dance_move() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        oricorio().unwrap(),
        oricorio().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oricorio,player-1,1|name:Swords Dance|target:Oricorio,player-1,1",
            "boost|mon:Oricorio,player-1,1|stat:atk|by:2",
            "activate|mon:Oricorio,player-2,1|ability:Dancer",
            "move|mon:Oricorio,player-2,1|name:Swords Dance|target:Oricorio,player-2,1|from:ability:Dancer",
            "boost|mon:Oricorio,player-2,1|stat:atk|by:2",
            "move|mon:Oricorio,player-2,1|name:Tackle|target:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:98/135",
            "damage|mon:Oricorio,player-1,1|health:73/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dancer_does_not_copy_unsuccessful_move() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        oricorio().unwrap(),
        oricorio().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oricorio,player-1,1|name:Protect|target:Oricorio,player-1,1",
            "singleturn|mon:Oricorio,player-1,1|move:Protect",
            "move|mon:Oricorio,player-2,1|name:Fiery Dance|target:Oricorio,player-1,1",
            "activate|mon:Oricorio,player-1,1|move:Protect",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Oricorio,player-1,1|name:Tackle|target:Oricorio,player-2,1",
            "split|side:1",
            "damage|mon:Oricorio,player-2,1|health:116/135",
            "damage|mon:Oricorio,player-2,1|health:86/100",
            "move|mon:Oricorio,player-2,1|name:Fiery Dance|target:Oricorio,player-1,1",
            "resisted|mon:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:98/135",
            "damage|mon:Oricorio,player-1,1|health:73/100",
            "boost|mon:Oricorio,player-2,1|stat:spa|by:1",
            "activate|mon:Oricorio,player-1,1|ability:Dancer",
            "move|mon:Oricorio,player-1,1|name:Fiery Dance|target:Oricorio,player-2,1|from:ability:Dancer",
            "resisted|mon:Oricorio,player-2,1",
            "split|side:1",
            "damage|mon:Oricorio,player-2,1|health:79/135",
            "damage|mon:Oricorio,player-2,1|health:59/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dancer_copies_move_and_targets_original_foe_in_doubles() {
    let mut battle = make_battle(
        0,
        BattleType::Doubles,
        oricorio().unwrap(),
        oricorio().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oricorio,player-1,1|name:Fiery Dance|target:Oricorio,player-2,2",
            "split|side:1",
            "damage|mon:Oricorio,player-2,2|health:60/135",
            "damage|mon:Oricorio,player-2,2|health:45/100",
            "activate|mon:Oricorio,player-2,2|ability:Dancer",
            "move|mon:Oricorio,player-2,2|name:Fiery Dance|target:Oricorio,player-1,1|from:ability:Dancer",
            "resisted|mon:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:110/135",
            "damage|mon:Oricorio,player-1,1|health:82/100",
            "boost|mon:Oricorio,player-2,2|stat:spa|by:1",
            "activate|mon:Oricorio,player-2,1|ability:Dancer",
            "move|mon:Oricorio,player-2,1|name:Fiery Dance|target:Oricorio,player-1,1|from:ability:Dancer",
            "resisted|mon:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:73/135",
            "damage|mon:Oricorio,player-1,1|health:55/100",
            "activate|mon:Oricorio,player-1,2|ability:Dancer",
            "move|mon:Oricorio,player-1,2|name:Fiery Dance|target:Oricorio,player-2,2|from:ability:Dancer",
            "split|side:1",
            "damage|mon:Oricorio,player-2,2|health:10/135",
            "damage|mon:Oricorio,player-2,2|health:8/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dancer_reactivates_confusion() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        oricorio().unwrap(),
        oricorio().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oricorio,player-2,1|name:Confuse Ray|target:Oricorio,player-1,1",
            "start|mon:Oricorio,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:2",
            "continue",
            "activate|mon:Oricorio,player-1,1|condition:Confusion",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|from:Confusion|health:116/135",
            "damage|mon:Oricorio,player-1,1|from:Confusion|health:86/100",
            "move|mon:Oricorio,player-2,1|name:Fiery Dance|target:Oricorio,player-1,1",
            "resisted|mon:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:79/135",
            "damage|mon:Oricorio,player-1,1|health:59/100",
            "boost|mon:Oricorio,player-2,1|stat:spa|by:1",
            "activate|mon:Oricorio,player-1,1|ability:Dancer",
            "end|mon:Oricorio,player-1,1|condition:Confusion",
            "move|mon:Oricorio,player-1,1|name:Fiery Dance|target:Oricorio,player-2,1|from:ability:Dancer",
            "resisted|mon:Oricorio,player-2,1",
            "split|side:1",
            "damage|mon:Oricorio,player-2,1|health:98/135",
            "damage|mon:Oricorio,player-2,1|health:73/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dancer_fails_when_locked_into_move() {
    let mut team = oricorio().unwrap();
    team.members[0].item = Some("Eject Button".to_owned());
    let mut battle = make_battle(0, BattleType::Singles, team, oricorio().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oricorio,player-1,1|name:Petal Dance|target:Oricorio,player-2,1",
            "resisted|mon:Oricorio,player-2,1",
            "split|side:1",
            "damage|mon:Oricorio,player-2,1|health:117/135",
            "damage|mon:Oricorio,player-2,1|health:87/100",
            "activate|mon:Oricorio,player-2,1|ability:Dancer",
            "move|mon:Oricorio,player-2,1|name:Petal Dance|target:Oricorio,player-1,1|from:ability:Dancer",
            "resisted|mon:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:117/135",
            "damage|mon:Oricorio,player-1,1|health:87/100",
            "itemend|mon:Oricorio,player-1,1|item:Eject Button",
            "switchout|mon:Oricorio,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Oricorio"],
            ["switch", "player-1", "Oricorio"],
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Oricorio,player-1,1|name:Swords Dance|target:Oricorio,player-1,1",
            "boost|mon:Oricorio,player-1,1|stat:atk|by:2",
            "activate|mon:Oricorio,player-2,1|ability:Dancer",
            "cant|mon:Oricorio,player-2,1|from:move:Petal Dance",
            "move|mon:Oricorio,player-2,1|name:Petal Dance|target:Oricorio,player-1,1",
            "resisted|mon:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:98/135",
            "damage|mon:Oricorio,player-1,1|health:73/100",
            "start|mon:Oricorio,player-2,1|condition:Confusion|fatigue",
            "activate|mon:Oricorio,player-1,1|ability:Dancer",
            "move|mon:Oricorio,player-1,1|name:Petal Dance|target:Oricorio,player-2,1|from:ability:Dancer",
            "resisted|mon:Oricorio,player-2,1",
            "split|side:1",
            "damage|mon:Oricorio,player-2,1|health:99/135",
            "damage|mon:Oricorio,player-2,1|health:74/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
