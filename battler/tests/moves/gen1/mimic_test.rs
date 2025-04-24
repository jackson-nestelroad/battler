use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,

    Id,
    LocalDataStore,
    MonMoveSlotData,
    MoveTarget,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Wigglytuff",
                    "species": "Wigglytuff",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Mimic"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Flareon",
                    "species": "Flareon",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Flamethrower",
                        "Quick Attack"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Jolteon",
                    "species": "Jolteon",
                    "ability": "No Ability",
                    "moves": [],
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
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn mimic_overwrites_move_slot_as_volatile() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 2,1"),
        Ok(())
    );

    assert!(battle
        .request_for_player("player-1")
        .is_some_and(|request| match request {
            Request::Turn(request) => request.active.first().is_some_and(|mon| {
                pretty_assertions::assert_eq!(
                    mon.moves.get(1),
                    Some(&MonMoveSlotData {
                        name: "Quick Attack".to_owned(),
                        id: Id::from("quickattack"),
                        pp: 30,
                        max_pp: 30,
                        target: Some(MoveTarget::Normal),
                        disabled: false,
                    })
                );
                true
            }),
            _ => false,
        }));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 2,1"),
        Ok(())
    );

    assert!(battle
        .request_for_player("player-1")
        .is_some_and(|request| match request {
            Request::Turn(request) => request.active.first().is_some_and(|mon| {
                pretty_assertions::assert_eq!(
                    mon.moves.get(1),
                    Some(&MonMoveSlotData {
                        name: "Quick Attack".to_owned(),
                        id: Id::from("quickattack"),
                        pp: 29,
                        max_pp: 30,
                        target: Some(MoveTarget::Normal),
                        disabled: false,
                    })
                );
                true
            }),
            _ => false,
        }));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 1,1"),
        Ok(())
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 0;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    assert!(battle
        .request_for_player("player-1")
        .is_some_and(|request| match request {
            Request::Turn(request) => request.active.first().is_some_and(|mon| {
                pretty_assertions::assert_eq!(
                    mon.moves.get(1),
                    Some(&MonMoveSlotData {
                        name: "Mimic".to_owned(),
                        id: Id::from("mimic"),
                        pp: 8,
                        max_pp: 10,
                        target: Some(MoveTarget::Normal),
                        disabled: false,
                    })
                );
                true
            }),
            _ => false,
        }));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    assert!(battle
        .request_for_player("player-1")
        .is_some_and(|request| match request {
            Request::Turn(request) => request.active.first().is_some_and(|mon| {
                pretty_assertions::assert_eq!(
                    mon.moves.get(1),
                    Some(&MonMoveSlotData {
                        name: "Flamethrower".to_owned(),
                        id: Id::from("flamethrower"),
                        pp: 15,
                        max_pp: 15,
                        target: Some(MoveTarget::Normal),
                        disabled: false,
                    })
                );
                true
            }),
            _ => false,
        }));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.player_data("player-1").unwrap().mons[0]
            .summary
            .moves[1]
            .pp,
        7
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wigglytuff,player-1,1|name:Mimic|noanim",
            "fail|mon:Wigglytuff,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Flareon,player-2,2|name:Quick Attack|target:Wigglytuff,player-1,1",
            "split|side:0",
            "damage|mon:Wigglytuff,player-1,1|health:153/200",
            "damage|mon:Wigglytuff,player-1,1|health:77/100",
            "move|mon:Wigglytuff,player-1,1|name:Mimic|target:Flareon,player-2,2",
            "start|mon:Flareon,player-2,2|move:Mimic|mimic:Quick Attack",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Flareon,player-2,2|name:Quick Attack|target:Wigglytuff,player-1,1",
            "split|side:0",
            "damage|mon:Wigglytuff,player-1,1|health:109/200",
            "damage|mon:Wigglytuff,player-1,1|health:55/100",
            "move|mon:Wigglytuff,player-1,1|name:Quick Attack|target:Flareon,player-2,2",
            "split|side:1",
            "damage|mon:Flareon,player-2,2|health:98/125",
            "damage|mon:Flareon,player-2,2|health:79/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Jolteon"],
            ["switch", "player-1", "Jolteon"],
            "move|mon:Flareon,player-2,2|name:Flamethrower|target:Jolteon,player-1,1",
            "split|side:0",
            "damage|mon:Jolteon,player-1,1|health:70/125",
            "damage|mon:Jolteon,player-1,1|health:56/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Wigglytuff"],
            ["switch", "player-1", "Wigglytuff"],
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Wigglytuff,player-1,1|name:Mimic|target:Flareon,player-2,2",
            "start|mon:Flareon,player-2,2|move:Mimic|mimic:Flamethrower",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Wigglytuff,player-1,1|name:Flamethrower|target:Flareon,player-2,2",
            "resisted|mon:Flareon,player-2,2",
            "split|side:1",
            "damage|mon:Flareon,player-2,2|health:83/125",
            "damage|mon:Flareon,player-2,2|health:67/100",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mimic_fails_on_moves_marked_fail_mimic() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 1,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;switch 2"),
        Ok(())
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Jolteon"],
            ["switch", "player-2", "Jolteon"],
            "move|mon:Flareon,player-1,2|name:Flamethrower|target:Wigglytuff,player-2,1",
            "split|side:1",
            "damage|mon:Wigglytuff,player-2,1|health:94/200",
            "damage|mon:Wigglytuff,player-2,1|health:47/100",
            "move|mon:Wigglytuff,player-2,1|name:Mimic|target:Flareon,player-1,2",
            "start|mon:Flareon,player-1,2|move:Mimic|mimic:Flamethrower",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Wigglytuff,player-1,1|name:Mimic|noanim",
            "fail|mon:Wigglytuff,player-1,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Jolteon,player-2,2|name:Struggle|target:Wigglytuff,player-1,1",
            "split|side:0",
            "damage|mon:Wigglytuff,player-1,1|health:171/200",
            "damage|mon:Wigglytuff,player-1,1|health:86/100",
            "split|side:1",
            "damage|mon:Jolteon,player-2,2|from:Struggle Recoil|health:94/125",
            "damage|mon:Jolteon,player-2,2|from:Struggle Recoil|health:76/100",
            "move|mon:Wigglytuff,player-1,1|name:Mimic|noanim",
            "fail|mon:Wigglytuff,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
