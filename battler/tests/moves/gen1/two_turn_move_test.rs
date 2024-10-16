use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
        Request,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn two_pidgeot() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pidgeot",
                    "species": "Pidgeot",
                    "ability": "No Ability",
                    "moves": [
                        "Razor Wind",
                        "Fly",
                        "Gust",
                        "Quick Attack"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Pidgeot",
                    "species": "Pidgeot",
                    "ability": "No Ability",
                    "moves": [
                        "Razor Wind",
                        "Fly",
                        "Gust",
                        "Quick Attack"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blastoise() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "No Ability",
                    "moves": [
                        "Skull Bash"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_volatile_status_logs(true)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn razor_wind_uses_two_turns() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        10002323,
        two_pidgeot().unwrap(),
        two_pidgeot().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_lock_move_request = serde_json::from_str(
        r#"{
            "team_position": 0,
            "moves": [
                {
                    "name": "Razor Wind",
                    "id": "razorwind",
                    "pp": 0,
                    "max_pp": 0,
                    "disabled": false
                }
            ],
            "trapped": true
        }"#,
    )
    .unwrap();
    assert_eq!(
        battle
            .request_for_player("player-1")
            .map(|req| if let Request::Turn(req) = req {
                req.active.get(0).cloned()
            } else {
                None
            })
            .flatten(),
        Some(expected_lock_move_request)
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1"),
        Err(err) => assert_eq!(err.full_description(), "cannot move: Pidgeot does not have a move in slot 1")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1"),
        Err(err) => assert_eq!(err.full_description(), "cannot switch: Pidgeot is trapped")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(
        battle
            .request_for_player("player-1")
            .map(|req| {
                if let Request::Turn(req) = req {
                    Some(req.active.get(0)?.moves.get(0)?.pp)
                } else {
                    None
                }
            })
            .flatten(),
        Some(9)
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pidgeot,player-1,1|name:Razor Wind|noanim",
            "prepare|mon:Pidgeot,player-1,1|move:Razor Wind",
            "addvolatile|mon:Pidgeot,player-1,1|volatile:Razor Wind|from:Two Turn Move",
            "addvolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:move:Razor Wind",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Razor Wind",
            "removevolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:move:Razor Wind",
            "removevolatile|mon:Pidgeot,player-1,1|volatile:Razor Wind|from:move:Razor Wind",
            "split|side:1",
            "damage|mon:Pidgeot,player-2,1|health:89/143",
            "damage|mon:Pidgeot,player-2,1|health:63/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fly_grants_invulnerability() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        60528764357287,
        two_pidgeot().unwrap(),
        two_pidgeot().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_lock_move_request = serde_json::from_str(
        r#"{
            "team_position": 0,
            "moves": [
                {
                    "name": "Fly",
                    "id": "fly",
                    "pp": 0,
                    "max_pp": 0,
                    "disabled": false
                }
            ],
            "trapped": true
        }"#,
    )
    .unwrap();
    assert_eq!(
        battle
            .request_for_player("player-1")
            .map(|req| if let Request::Turn(req) = req {
                req.active.get(0).cloned()
            } else {
                None
            })
            .flatten(),
        Some(expected_lock_move_request)
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1"),
        Err(err) => assert_eq!(err.full_description(), "cannot move: Pidgeot does not have a move in slot 1")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1"),
        Err(err) => assert_eq!(err.full_description(), "cannot switch: Pidgeot is trapped")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    assert_matches::assert_matches!(
        battle
            .request_for_player("player-1")
            .map(|req| {
                if let Request::Turn(req) = req {
                    Some(req.active.get(0)?.moves.get(1)?.pp)
                } else {
                    None
                }
            })
            .flatten(),
        Some(14)
    );

    // Show Gust can hit Mons in Fly, and gains double power.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pidgeot,player-1,1|name:Fly|noanim",
            "prepare|mon:Pidgeot,player-1,1|move:Fly",
            "addvolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:Two Turn Move",
            "addvolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:move:Fly",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pidgeot,player-2,1|name:Quick Attack|noanim",
            "miss|mon:Pidgeot,player-1,1",
            "move|mon:Pidgeot,player-1,1|name:Fly|target:Pidgeot,player-2,1",
            "removevolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:move:Fly",
            "removevolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:move:Fly",
            "split|side:1",
            "damage|mon:Pidgeot,player-2,1|health:80/143",
            "damage|mon:Pidgeot,player-2,1|health:56/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Fly|noanim",
            "prepare|mon:Pidgeot,player-1,1|move:Fly",
            "addvolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:Two Turn Move",
            "addvolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:move:Fly",
            "move|mon:Pidgeot,player-2,1|name:Gust|target:Pidgeot,player-1,1",
            "split|side:0",
            "damage|mon:Pidgeot,player-1,1|health:93/143",
            "damage|mon:Pidgeot,player-1,1|health:66/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fly_locks_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Doubles,
        0,
        two_pidgeot().unwrap(),
        two_pidgeot().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_lock_move_request = serde_json::from_str(
        r#"{
            "team_position": 0,
            "moves": [
                {
                    "name": "Fly",
                    "id": "fly",
                    "pp": 0,
                    "max_pp": 0,
                    "disabled": false
                }
            ],
            "trapped": false
        }"#,
    )
    .unwrap();
    assert_eq!(
        battle
            .request_for_player("player-1")
            .map(|req| if let Request::Turn(req) = req {
                req.active.get(0).cloned()
            } else {
                None
            })
            .flatten(),
        Some(expected_lock_move_request)
    );

    // This target is ignored.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pidgeot,player-1,1|name:Fly|noanim",
            "prepare|mon:Pidgeot,player-1,1|move:Fly",
            "addvolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:Two Turn Move",
            "addvolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:move:Fly",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Fly|target:Pidgeot,player-2,2",
            "removevolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:move:Fly",
            "removevolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:move:Fly",
            "split|side:1",
            "damage|mon:Pidgeot,player-2,2|health:80/143",
            "damage|mon:Pidgeot,player-2,2|health:56/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn skull_bash_also_boosts_defense() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        blastoise().unwrap(),
        blastoise().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blastoise,player-1,1|name:Skull Bash|noanim",
            "prepare|mon:Blastoise,player-1,1|move:Skull Bash",
            "boost|mon:Blastoise,player-1,1|stat:def|by:1",
            "addvolatile|mon:Blastoise,player-1,1|volatile:Skull Bash|from:Two Turn Move",
            "addvolatile|mon:Blastoise,player-1,1|volatile:Two Turn Move|from:move:Skull Bash",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-1,1|name:Skull Bash|target:Blastoise,player-2,1",
            "removevolatile|mon:Blastoise,player-1,1|volatile:Two Turn Move|from:move:Skull Bash",
            "removevolatile|mon:Blastoise,player-1,1|volatile:Skull Bash|from:move:Skull Bash",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:92/139",
            "damage|mon:Blastoise,player-2,1|health:67/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
