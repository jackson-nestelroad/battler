use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    Type,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Dragapult",
                    "species": "Dragapult",
                    "ability": "No Ability",
                    "moves": [
                        "Dragon Darts",
                        "Protect",
                        "Splash",
                        "Fly",
                        "Copycat",
                        "Follow Me"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Dragapult",
                    "species": "Dragapult",
                    "ability": "No Ability",
                    "moves": [
                        "Dragon Darts",
                        "Protect",
                        "Ally Switch"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Dragapult",
                    "species": "Dragapult",
                    "ability": "No Ability",
                    "moves": [
                        "Dragon Darts"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn dragon_darts_targets_other_target_in_doubles() {
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,1",
            "supereffective|mon:Dragapult,player-2,1",
            "split|side:1",
            "damage|mon:Dragapult,player-2,1|health:92/286",
            "damage|mon:Dragapult,player-2,1|health:33/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:104/286",
            "damage|mon:Dragapult,player-2,2|health:37/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_hits_same_target_due_to_accuracy() {
    let mut battle = make_battle(
        123456,
        BattleType::Doubles,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.push_outside_effect(
            serde_json::from_str(
                r#"{
                    "name": "Evasion Boost",
                    "target": {
                        "mon": {
                            "player": "player-2",
                            "position": 0
                        }
                    },
                    "program": [
                        "boost: $target 'eva:6'"
                    ]
                }"#,
            )
            .unwrap(),
        ),
        Ok(())
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "boost|mon:Dragapult,player-2,1|stat:eva|by:6|from:Evasion Boost",
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:94/286",
            "damage|mon:Dragapult,player-2,2|health:33/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:0",
            "damage|mon:Dragapult,player-2,2|health:0",
            "faint|mon:Dragapult,player-2,2",
            "hitcount|hits:2",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_hits_same_target_due_to_immunity() {
    let mut team_2 = team().unwrap();
    team_2.members[0].tera_type = Some(Type::Fairy);
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,tera;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Dragapult,player-2,1|type:Fairy",
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:92/286",
            "damage|mon:Dragapult,player-2,2|health:33/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:0",
            "damage|mon:Dragapult,player-2,2|health:0",
            "faint|mon:Dragapult,player-2,2",
            "hitcount|hits:2",
            "move|mon:Dragapult,player-2,1|name:Splash|target:Dragapult,player-2,1",
            "activate|move:Splash",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_hits_same_target_due_to_protect() {
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dragapult,player-2,2|name:Protect|target:Dragapult,player-2,2",
            "singleturn|mon:Dragapult,player-2,2|move:Protect",
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,1",
            "supereffective|mon:Dragapult,player-2,1",
            "split|side:1",
            "damage|mon:Dragapult,player-2,1|health:92/286",
            "damage|mon:Dragapult,player-2,1|health:33/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,1",
            "supereffective|mon:Dragapult,player-2,1",
            "split|side:1",
            "damage|mon:Dragapult,player-2,1|health:0",
            "damage|mon:Dragapult,player-2,1|health:0",
            "faint|mon:Dragapult,player-2,1",
            "hitcount|hits:2",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_hits_same_target_due_to_invulnerability() {
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dragapult,player-1,1|name:Fly|noanim",
            "prepare|mon:Dragapult,player-1,1|move:Fly",
            "move|mon:Dragapult,player-2,1|name:Dragon Darts|target:Dragapult,player-1,2",
            "supereffective|mon:Dragapult,player-1,2",
            "split|side:0",
            "damage|mon:Dragapult,player-1,2|health:92/286",
            "damage|mon:Dragapult,player-1,2|health:33/100",
            "animatemove|mon:Dragapult,player-2,1|name:Dragon Darts|target:Dragapult,player-1,2",
            "supereffective|mon:Dragapult,player-1,2",
            "split|side:0",
            "damage|mon:Dragapult,player-1,2|health:0",
            "damage|mon:Dragapult,player-1,2|health:0",
            "faint|mon:Dragapult,player-1,2",
            "hitcount|hits:2",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_hits_same_target_due_to_ability_immunity() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Wonder Guard".to_owned();
    team_2.members[0].tera_type = Some(Type::Fire);
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,tera;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Dragapult,player-2,1|type:Fire",
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:92/286",
            "damage|mon:Dragapult,player-2,2|health:33/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:0",
            "damage|mon:Dragapult,player-2,2|health:0",
            "faint|mon:Dragapult,player-2,2",
            "hitcount|hits:2",
            "move|mon:Dragapult,player-2,1|name:Splash|target:Dragapult,player-2,1",
            "activate|move:Splash",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_hits_same_target_due_to_prankster_immunity() {
    let mut team_1 = team().unwrap();
    team_1.members[0].ability = "Prankster".to_owned();
    let mut team_2 = team().unwrap();
    team_2.members[0].tera_type = Some(Type::Dark);
    let mut battle = make_battle(0, BattleType::Doubles, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1;move 1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1,tera;pass"),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Dragapult,player-2,1|type:Dark",
            "move|mon:Dragapult,player-1,1|name:Protect|target:Dragapult,player-1,1",
            "singleturn|mon:Dragapult,player-1,1|move:Protect",
            "move|mon:Dragapult,player-1,2|name:Protect|target:Dragapult,player-1,2",
            "singleturn|mon:Dragapult,player-1,2|move:Protect",
            "move|mon:Dragapult,player-2,1|name:Dragon Darts|noanim",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Dragapult,player-1,1|name:Copycat|target:Dragapult,player-1,1",
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2|from:move:Copycat",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:92/286",
            "damage|mon:Dragapult,player-2,2|health:33/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:0",
            "damage|mon:Dragapult,player-2,2|health:0",
            "faint|mon:Dragapult,player-2,2",
            "hitcount|hits:2",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_honors_follow_me() {
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dragapult,player-2,1|name:Follow Me|target:Dragapult,player-2,1",
            "singleturn|mon:Dragapult,player-2,1|move:Follow Me",
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,1",
            "supereffective|mon:Dragapult,player-2,1",
            "split|side:1",
            "damage|mon:Dragapult,player-2,1|health:92/286",
            "damage|mon:Dragapult,player-2,1|health:33/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,1",
            "supereffective|mon:Dragapult,player-2,1",
            "split|side:1",
            "damage|mon:Dragapult,player-2,1|health:0",
            "damage|mon:Dragapult,player-2,1|health:0",
            "faint|mon:Dragapult,player-2,1",
            "hitcount|hits:2",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_does_not_target_self_as_second_target() {
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-1,2",
            "supereffective|mon:Dragapult,player-1,2",
            "split|side:0",
            "damage|mon:Dragapult,player-1,2|health:92/286",
            "damage|mon:Dragapult,player-1,2|health:33/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-1,2",
            "supereffective|mon:Dragapult,player-1,2",
            "split|side:0",
            "damage|mon:Dragapult,player-1,2|health:0",
            "damage|mon:Dragapult,player-1,2|health:0",
            "faint|mon:Dragapult,player-1,2",
            "hitcount|hits:2",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_does_not_hit_self_due_to_ally_switch() {
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;move 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dragapult,player-1,2|name:Ally Switch|target:Dragapult,player-1,2",
            "swap|mon:Dragapult,player-1,2|position:1|from:move:Ally Switch",
            "move|mon:Dragapult,player-1,2|name:Dragon Darts|target:Dragapult,player-2,1",
            "supereffective|mon:Dragapult,player-2,1",
            "split|side:1",
            "damage|mon:Dragapult,player-2,1|health:82/286",
            "damage|mon:Dragapult,player-2,1|health:29/100",
            "animatemove|mon:Dragapult,player-1,2|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:104/286",
            "damage|mon:Dragapult,player-2,2|health:37/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_darts_iterates_through_adjacent_targets_in_triples() {
    let mut team_1 = team().unwrap();
    team_1.members[0].level = 1;
    team_1.members[1].level = 1;
    let mut battle = make_battle(0, BattleType::Triples, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.push_outside_condition(
            serde_json::from_str(
                r#"{
                    "name": "Multi-Darts",
                    "condition_type": "Condition",
                    "condition": {
                        "callbacks": {
                            "on_use_move": ["if $move.id == dragondarts:", ["$move.multihit = 6"]]
                        }
                    }
                }"#,
            )
            .unwrap(),
        ),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.push_outside_effect(
            serde_json::from_str(
                r#"{
                    "name": "Multi-Darts",
                    "target": {
                        "side": {
                            "index": 0
                        }
                    },
                    "program": [
                        "foreach $mon in func_call(all_active_mons_on_side: $side):",
                        ["add_volatile: $mon multidarts"] 
                    ]
                }"#,
            )
            .unwrap(),
        ),
        Ok(())
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:284/286",
            "damage|mon:Dragapult,player-2,2|health:99/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,3",
            "supereffective|mon:Dragapult,player-2,3",
            "split|side:1",
            "damage|mon:Dragapult,player-2,3|health:284/286",
            "damage|mon:Dragapult,player-2,3|health:99/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:282/286",
            "damage|mon:Dragapult,player-2,2|health:99/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,3",
            "supereffective|mon:Dragapult,player-2,3",
            "split|side:1",
            "damage|mon:Dragapult,player-2,3|health:282/286",
            "damage|mon:Dragapult,player-2,3|health:99/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:280/286",
            "damage|mon:Dragapult,player-2,2|health:98/100",
            "animatemove|mon:Dragapult,player-1,1|name:Dragon Darts|target:Dragapult,player-2,3",
            "supereffective|mon:Dragapult,player-2,3",
            "split|side:1",
            "damage|mon:Dragapult,player-2,3|health:280/286",
            "damage|mon:Dragapult,player-2,3|health:98/100",
            "hitcount|hits:6",
            "move|mon:Dragapult,player-1,2|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:278/286",
            "damage|mon:Dragapult,player-2,2|health:98/100",
            "animatemove|mon:Dragapult,player-1,2|name:Dragon Darts|target:Dragapult,player-2,1",
            "supereffective|mon:Dragapult,player-2,1",
            "split|side:1",
            "damage|mon:Dragapult,player-2,1|health:284/286",
            "damage|mon:Dragapult,player-2,1|health:99/100",
            "animatemove|mon:Dragapult,player-1,2|name:Dragon Darts|target:Dragapult,player-2,3",
            "supereffective|mon:Dragapult,player-2,3",
            "split|side:1",
            "damage|mon:Dragapult,player-2,3|health:278/286",
            "damage|mon:Dragapult,player-2,3|health:98/100",
            "animatemove|mon:Dragapult,player-1,2|name:Dragon Darts|target:Dragapult,player-2,2",
            "supereffective|mon:Dragapult,player-2,2",
            "split|side:1",
            "damage|mon:Dragapult,player-2,2|health:276/286",
            "damage|mon:Dragapult,player-2,2|health:97/100",
            "animatemove|mon:Dragapult,player-1,2|name:Dragon Darts|target:Dragapult,player-2,1",
            "supereffective|mon:Dragapult,player-2,1",
            "split|side:1",
            "damage|mon:Dragapult,player-2,1|health:282/286",
            "damage|mon:Dragapult,player-2,1|health:99/100",
            "animatemove|mon:Dragapult,player-1,2|name:Dragon Darts|target:Dragapult,player-2,3",
            "supereffective|mon:Dragapult,player-2,3",
            "split|side:1",
            "damage|mon:Dragapult,player-2,3|health:272/286",
            "damage|mon:Dragapult,player-2,3|health:96/100",
            "hitcount|hits:6",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
