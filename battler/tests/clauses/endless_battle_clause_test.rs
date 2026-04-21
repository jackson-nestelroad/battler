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

fn can_cause_endless_battle_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Slowbro",
                    "species": "Slowbro",
                    "ability": "No Ability",
                    "item": "Leppa Berry",
                    "moves": [
                        "Recycle",
                        "Heal Pulse",
                        "Slack Off",
                        "Block"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn cannot_cause_endless_battle_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [
                        "Pound",
                        "Growl"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": [
                        "Pound"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "No Ability",
                    "moves": [
                        "Pound"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

#[track_caller]
fn turn(battle: &mut PublicCoreBattle, player_1: &str, player_2: &str) {
    assert_matches::assert_matches!(battle.set_player_choice("player-1", player_1), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", player_2), Ok(()));
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .with_rule("Endless Battle Clause")
        .build(static_local_data_store())
}

#[test]
fn detects_endless_battle_initiated_by_single_player() {
    let mut battle = make_battle(
        0,
        can_cause_endless_battle_team().unwrap(),
        cannot_cause_endless_battle_team().unwrap(),
    )
    .unwrap();

    assert_matches::assert_matches!(battle.validate_player("player-1"), Ok(()));
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1.
    turn(&mut battle, "move 3", "move 1");

    for i in 0..6 {
        // Turns 2-6, 8-12, 14-18, ...
        for _ in 0..5 {
            turn(&mut battle, "move 2", "move 0");
        }
        // Turns 7, 13, 19, ...
        if i != 5 {
            turn(&mut battle, "move 0", "move 0");
        }
    }

    // Turn 37.
    turn(&mut battle, "move 0", "move 1");

    // Turns 38-75.
    for _ in 0..38 {
        turn(&mut battle, "move 0", "move 1");
    }

    for i in 0..3 {
        // Turns 76-79, 86-89, 96-99.
        for _ in 0..4 {
            turn(&mut battle, "move 1", "move 0");
        }

        // Turn 80, 90, 100.
        turn(&mut battle, "move 2", "move 0");

        // Turn 101: Endless Battle Clause triggered.
        if i == 2 {
            break;
        }

        // Turn 81-83, 91-93.
        for _ in 0..3 {
            turn(&mut battle, "move 1", "move 0");
        }
        // Turn 84, 94.
        turn(&mut battle, "move 2", "move 0");
        // Turn 85, 95.
        turn(&mut battle, "move 0", "move 0");
    }

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-2,1|name:Struggle|target:Slowbro,player-1,1",
            "split|side:0",
            "damage|mon:Slowbro,player-1,1|health:83/155",
            "damage|mon:Slowbro,player-1,1|health:54/100",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|from:Struggle Recoil|health:79/105",
            "damage|mon:Bulbasaur,player-2,1|from:Struggle Recoil|health:76/100",
            "move|mon:Slowbro,player-1,1|name:Slack Off|target:Slowbro,player-1,1",
            "split|side:0",
            "heal|mon:Slowbro,player-1,1|health:155/155",
            "heal|mon:Slowbro,player-1,1|health:100/100",
            "itemend|mon:Slowbro,player-1,1|item:Leppa Berry|eat",
            "restorepp|mon:Slowbro,player-1,1|move:Slack Off|by:5|from:item:Leppa Berry",
            "residual",
            "activate|clause:Endless Battle Clause|sides:0",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 100, &expected_logs);
}

#[test]
fn endless_battle_clause_does_not_activate_if_mon_can_switch() {
    // Offender can switch.
    let mut team_1 = can_cause_endless_battle_team().unwrap();
    team_1.members.push(team_1.members[0].clone());

    // Make Mon immune to trapping.
    let mut team_2 = cannot_cause_endless_battle_team().unwrap();
    team_2.members[0].name = "Gastly".to_owned();
    team_2.members[0].species = "Gastly".to_owned();

    let mut battle = make_battle(0, team_1, team_2).unwrap();

    assert_matches::assert_matches!(battle.validate_player("player-1"), Ok(()));
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1.
    turn(&mut battle, "move 3", "move 1");

    for i in 0..6 {
        // Turns 2-6, 8-12, 14-18, ...
        for _ in 0..5 {
            turn(&mut battle, "move 2", "move 0");
        }
        // Turns 7, 13, 19, ...
        if i != 5 {
            turn(&mut battle, "move 0", "move 0");
        }
    }

    // Turn 37.
    turn(&mut battle, "move 0", "move 1");

    // Turns 38-75.
    for _ in 0..38 {
        turn(&mut battle, "move 0", "move 1");
    }

    for i in 0..3 {
        // Turns 76-79, 86-89, 96-99.
        for _ in 0..4 {
            turn(&mut battle, "move 1", "move 0");
        }

        // Turn 80, 90, 100.
        turn(&mut battle, "move 2", "move 0");

        // Turn 101: Endless Battle Clause triggered.
        if i == 2 {
            break;
        }

        // Turn 81-83, 91-93.
        for _ in 0..3 {
            turn(&mut battle, "move 1", "move 0");
        }
        // Turn 84, 94.
        turn(&mut battle, "move 2", "move 0");
        // Turn 85, 95.
        turn(&mut battle, "move 0", "move 0");
    }

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gastly,player-2,1|name:Struggle|target:Slowbro,player-1,1",
            "split|side:0",
            "damage|mon:Slowbro,player-1,1|health:101/155",
            "damage|mon:Slowbro,player-1,1|health:66/100",
            "split|side:1",
            "damage|mon:Gastly,player-2,1|from:Struggle Recoil|health:67/90",
            "damage|mon:Gastly,player-2,1|from:Struggle Recoil|health:75/100",
            "move|mon:Slowbro,player-1,1|name:Slack Off|target:Slowbro,player-1,1",
            "split|side:0",
            "heal|mon:Slowbro,player-1,1|health:155/155",
            "heal|mon:Slowbro,player-1,1|health:100/100",
            "itemend|mon:Slowbro,player-1,1|item:Leppa Berry|eat",
            "restorepp|mon:Slowbro,player-1,1|move:Slack Off|by:5|from:item:Leppa Berry",
            "residual",
            "turn|turn:101"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 100, &expected_logs);
}
