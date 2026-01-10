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

fn cloyster() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cloyster",
                    "species": "Cloyster",
                    "ability": "No Ability",
                    "moves": [
                        "Wonder Room",
                        "Tackle",
                        "Ice Beam",
                        "Psyshock",
                        "Body Press",
                        "Iron Defense",
                        "Quiver Dance"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "ivs": {
                        "def": 31,
                        "spd": 31
                    },
                    "evs": {
                        "def": 252,
                        "spd": 0
                    }
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn wonder_room_swaps_defense_and_special_defense() {
    let mut battle = make_battle(0, cloyster().unwrap(), cloyster().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cloyster,player-1,1|name:Tackle|target:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:195/210",
            "damage|mon:Cloyster,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Ice Beam|target:Cloyster,player-2,1",
            "resisted|mon:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:159/210",
            "damage|mon:Cloyster,player-2,1|health:76/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Wonder Room",
            "fieldstart|move:Wonder Room|of:Cloyster,player-1,1",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Tackle|target:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:106/210",
            "damage|mon:Cloyster,player-2,1|health:51/100",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Ice Beam|target:Cloyster,player-2,1",
            "resisted|mon:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:96/210",
            "damage|mon:Cloyster,player-2,1|health:46/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn wonder_room_affects_psyshock() {
    let mut battle = make_battle(0, cloyster().unwrap(), cloyster().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cloyster,player-1,1|name:Psyshock|target:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:184/210",
            "damage|mon:Cloyster,player-2,1|health:88/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Wonder Room",
            "fieldstart|move:Wonder Room|of:Cloyster,player-1,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Psyshock|target:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:99/210",
            "damage|mon:Cloyster,player-2,1|health:48/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn wonder_room_affects_body_press() {
    let mut battle = make_battle(0, cloyster().unwrap(), cloyster().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Body Press by default (attacker Def vs. defender Def).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "item maxpotion,-1"),
        Ok(())
    );

    // Body Press under Wonder Room (attacker SpD vs. defender SpD).
    //
    // Damage should be the same.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Boost SpD, which is taken into consideration under Wonder Room, only for the attacker.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 6"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 6"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "item maxpotion,-1"),
        Ok(())
    );

    // Body Press under Wonder Room (attacker SpD vs. defender SpD).
    //
    // Only attacker boosts are conisdered, so damage increases.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cloyster,player-1,1|name:Body Press|target:Cloyster,player-2,1",
            "supereffective|mon:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:78/210",
            "damage|mon:Cloyster,player-2,1|health:38/100",
            "residual",
            "turn|turn:2",
            "continue",
            "useitem|player:player-2|name:Max Potion|target:Cloyster,player-2,1",
            "split|side:1",
            "heal|mon:Cloyster,player-2,1|from:item:Max Potion|health:210/210",
            "heal|mon:Cloyster,player-2,1|from:item:Max Potion|health:100/100",
            "move|mon:Cloyster,player-1,1|name:Wonder Room",
            "fieldstart|move:Wonder Room|of:Cloyster,player-1,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Body Press|target:Cloyster,player-2,1",
            "supereffective|mon:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:86/210",
            "damage|mon:Cloyster,player-2,1|health:41/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Quiver Dance|target:Cloyster,player-1,1",
            "boost|mon:Cloyster,player-1,1|stat:spa|by:1",
            "boost|mon:Cloyster,player-1,1|stat:spd|by:1",
            "boost|mon:Cloyster,player-1,1|stat:spe|by:1",
            "move|mon:Cloyster,player-2,1|name:Quiver Dance|target:Cloyster,player-2,1",
            "boost|mon:Cloyster,player-2,1|stat:spa|by:1",
            "boost|mon:Cloyster,player-2,1|stat:spd|by:1",
            "boost|mon:Cloyster,player-2,1|stat:spe|by:1",
            "residual",
            "turn|turn:5",
            "continue",
            "useitem|player:player-2|name:Max Potion|target:Cloyster,player-2,1",
            "split|side:1",
            "heal|mon:Cloyster,player-2,1|from:item:Max Potion|health:210/210",
            "heal|mon:Cloyster,player-2,1|from:item:Max Potion|health:100/100",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Body Press|target:Cloyster,player-2,1",
            "supereffective|mon:Cloyster,player-2,1",
            "split|side:1",
            "damage|mon:Cloyster,player-2,1|health:38/210",
            "damage|mon:Cloyster,player-2,1|health:19/100",
            "fieldend|move:Wonder Room",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn wonder_room_terminates_after_5_turns() {
    let mut battle = make_battle(0, cloyster().unwrap(), cloyster().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    for _ in 0..4 {
        assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    }

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cloyster,player-1,1|name:Wonder Room",
            "fieldstart|move:Wonder Room|of:Cloyster,player-1,1",
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
            "fieldend|move:Wonder Room",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn wonder_room_removed_by_restart() {
    let mut battle = make_battle(0, cloyster().unwrap(), cloyster().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cloyster,player-1,1|name:Wonder Room",
            "fieldstart|move:Wonder Room|of:Cloyster,player-1,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cloyster,player-1,1|name:Wonder Room",
            "fieldend|move:Wonder Room",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
