use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    Id,
    MonMoveSlotData,
    MoveTarget,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn pikachu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Volt Tackle",
                        "Tackle",
                        "Thunderbolt",
                        "Tailwind",
                        "Swords Dance",
                        "Belly Drum",
                        "Memento",
                        "Destiny Bond",
                        "Splash",
                        "Hypnosis",
                        "Rain Dance",
                        "Weather Ball",
                        "Curse"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Raichu",
                    "species": "Raichu",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Mimikyu",
                    "species": "Mimikyu",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Curse"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn eevee() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Quick Attack",
                        "Water Gun",
                        "Thunder Wave",
                        "Protect",
                        "Disable",
                        "Mimic",
                        "Sketch",
                        "Spite",
                        "Me First"
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
        .with_z_moves(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn type_based_z_crystal_transforms_moves_of_same_type() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Electrium Z".to_owned());
    let mut eevee = eevee().unwrap();
    eevee.members[0].item = Some("Waterium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert_matches::assert_matches!(request.active[0].z_moves.get(0), Some(Some(data)) => {
            pretty_assertions::assert_eq!(
                *data,
                MonMoveSlotData {
                    id: Id::from("gigavolthavoc"),
                    name: "Gigavolt Havoc".to_owned(),
                    pp: 15,
                    max_pp: 15,
                    target: MoveTarget::Normal,
                    disabled: false,
                }
            );
        });
        assert_matches::assert_matches!(request.active[0].z_moves.get(2), Some(Some(data)) => {
            pretty_assertions::assert_eq!(
                *data,
                MonMoveSlotData {
                    id: Id::from("gigavolthavoc"),
                    name: "Gigavolt Havoc".to_owned(),
                    pp: 15,
                    max_pp: 15,
                    target: MoveTarget::Normal,
                    disabled: false,
                }
            );
        });
        assert!(
            request
                .active[0]
                .z_moves
                .iter()
                .enumerate()
                .all(|(i, z_move)| i == 0 || i == 2 || z_move.is_none()),
            "{:?}",
            request.active[0].z_moves
        );
    });
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        assert_matches::assert_matches!(request.active[0].z_moves.get(1), Some(Some(data)) => {
            pretty_assertions::assert_eq!(
                *data,
                MonMoveSlotData {
                    id: Id::from("hydrovortex"),
                    name: "Hydro Vortex".to_owned(),
                    pp: 25,
                    max_pp: 25,
                    target: MoveTarget::Normal,
                    disabled: false,
                }
            );
        });
        assert!(
            request
                .active[0]
                .z_moves
                .iter()
                .enumerate()
                .all(|(i, z_move)| i == 1 || z_move.is_none()),
            "{:?}",
            request.active[0].z_moves
        );
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: move in slot 1 cannot be upgraded to z-move");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2,zmove"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: move in slot 0 cannot be upgraded to z-move");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1,zmove"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Gigavolt Havoc|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:22/115",
            "damage|mon:Eevee,player-2,1|health:20/100",
            "singleturn|mon:Eevee,player-2,1|condition:Z-Power",
            "move|mon:Eevee,player-2,1|name:Hydro Vortex|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:53/95",
            "damage|mon:Pikachu,player-1,1|health:56/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn species_based_z_crystal_only_allows_single_move_and_user() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Pikanium Z".to_owned());
    let mut eevee = eevee().unwrap();
    eevee.members[0].item = Some("Pikanium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert_matches::assert_matches!(request.active[0].z_moves.get(0), Some(Some(data)) => {
            pretty_assertions::assert_eq!(
                *data,
                MonMoveSlotData {
                    id: Id::from("catastropika"),
                    name: "Catastropika".to_owned(),
                    pp: 15,
                    max_pp: 15,
                    target: MoveTarget::Normal,
                    disabled: false,
                }
            );
        });
        assert!(
            request
                .active[0]
                .z_moves
                .iter()
                .enumerate()
                .all(|(i, z_move)| i == 0 || z_move.is_none()),
            "{:?}",
            request.active[0].z_moves
        );
    });
    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_z_move, "{:?}", request.active[0]);
        assert!(
            request
                .active[0]
                .z_moves
                .iter()
                .all(|z_move| z_move.is_none()),
            "{:?}",
            request.active[0].z_moves
        );
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: move in slot 1 cannot be upgraded to z-move");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: move in slot 2 cannot be upgraded to z-move");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Eevee cannot z-move");
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Catastropika|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:0",
            "damage|mon:Eevee,player-2,1|health:0",
            "faint|mon:Eevee,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn player_can_z_move_once_per_battle() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_z_move, "{:?}", request.active[0]);
        assert!(
            request
                .active[0]
                .z_moves
                .iter()
                .all(|z_move| z_move.is_none()),
            "{:?}",
            request.active[0].z_moves
        );
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Err(err) => {
        assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Pikachu cannot z-move");
    });
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
}

#[test]
fn z_power_boosts_critical_hit_ratio() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Flyinium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Tailwind|zpower",
            "start|mon:Pikachu,player-1,1|move:Focus Energy",
            "sidestart|side:0|move:Tailwind",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn z_power_clears_negative_boosts() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Swords Dance|target:Pikachu,player-1,1|zpower",
            "clearnegativeboosts|mon:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:atk|by:2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn z_power_fully_heals() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 5,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-2,1|name:Quick Attack|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:58/95",
            "damage|mon:Pikachu,player-1,1|health:62/100",
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Belly Drum|target:Pikachu,player-1,1|zpower",
            "split|side:0",
            "heal|mon:Pikachu,player-1,1|health:95/95",
            "heal|mon:Pikachu,player-1,1|health:100/100",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:48/95",
            "damage|mon:Pikachu,player-1,1|health:51/100",
            "boost|mon:Pikachu,player-1,1|stat:atk|by:6|max",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn z_power_fully_heals_replacement() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Darkinium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 6,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Memento|target:Eevee,player-2,1|zpower",
            "unboost|mon:Eevee,player-2,1|stat:atk|by:2",
            "unboost|mon:Eevee,player-2,1|stat:spa|by:2",
            "faint|mon:Pikachu,player-1,1",
            "residual",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Raichu"],
            ["switch", "player-1", "Raichu"],
            "split|side:0",
            "heal|mon:Raichu,player-1,1|from:Heal Replacement|health:120/120",
            "heal|mon:Raichu,player-1,1|from:Heal Replacement|health:100/100",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 3, &expected_logs);
}

#[test]
fn z_power_redirects_moves() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Ghostium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Doubles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 7,0,zmove;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Destiny Bond|target:Pikachu,player-1,1|zpower",
            "singleturn|mon:Pikachu,player-1,1|move:Follow Me",
            "singlemove|mon:Pikachu,player-1,1|move:Destiny Bond",
            "move|mon:Eevee,player-2,1|name:Water Gun|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:77/95",
            "damage|mon:Pikachu,player-1,1|health:82/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn z_power_boosts_stats() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 8,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Splash|target:Pikachu,player-1,1|zpower",
            "boost|mon:Pikachu,player-1,1|stat:atk|by:3|from:Z-Power",
            "activate|move:Splash",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn z_power_applies_even_if_move_fails() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Psychium Z".to_owned());
    let mut battle = make_battle(0, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 9"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 9,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Hypnosis|target:Eevee,player-2,1",
            "status|mon:Eevee,player-2,1|status:Sleep",
            "residual",
            "turn|turn:2",
            "continue",
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Hypnosis|zpower|noanim",
            "boost|mon:Eevee,player-2,1|stat:spe|by:1|from:Z-Power|of:Pikachu,player-1,1",
            "fail|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn z_power_applies_even_if_move_fails_due_to_immunity() {
    let mut eevee = eevee().unwrap();
    eevee.members[0].item = Some("Electrium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu().unwrap(), eevee).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2,zmove"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Eevee,player-2,1|condition:Z-Power",
            "move|mon:Eevee,player-2,1|name:Thunder Wave|zpower|noanim",
            "boost|mon:Pikachu,player-1,1|stat:spd|by:1|from:Z-Power|of:Eevee,player-2,1",
            "immune|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn z_move_changes_based_on_move_with_dynamic_type() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 10"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 11,zmove"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Breakneck Blitz|noanim",
            "move|mon:Pikachu,player-1,1|name:Hydro Vortex|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:61/115",
            "damage|mon:Eevee,player-2,1|health:54/100",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn z_move_hits_through_protect() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Electrium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-2,1|name:Protect|target:Eevee,player-2,1",
            "singleturn|mon:Eevee,player-2,1|move:Protect",
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Gigavolt Havoc|target:Eevee,player-2,1",
            "protectweaken|mon:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:81/115",
            "damage|mon:Eevee,player-2,1|health:71/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn disable_fails_after_z_move() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Breakneck Blitz|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:65/115",
            "damage|mon:Eevee,player-2,1|health:57/100",
            "move|mon:Eevee,player-2,1|name:Disable|noanim",
            "fail|mon:Eevee,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mimic_fails_after_z_move() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Breakneck Blitz|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:65/115",
            "damage|mon:Eevee,player-2,1|health:57/100",
            "move|mon:Eevee,player-2,1|name:Mimic|noanim",
            "fail|mon:Eevee,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sketch_fails_after_z_move() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 6"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Breakneck Blitz|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:65/115",
            "damage|mon:Eevee,player-2,1|health:57/100",
            "move|mon:Eevee,player-2,1|name:Sketch|noanim",
            "fail|mon:Eevee,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn spite_deducts_pp_of_base_move() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 7"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Breakneck Blitz|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:65/115",
            "damage|mon:Eevee,player-2,1|health:57/100",
            "move|mon:Eevee,player-2,1|name:Spite|target:Pikachu,player-1,1",
            "deductpp|mon:Pikachu,player-1,1|move:Tackle|by:4",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn me_first_fails_for_z_move() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Normalium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 8"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Breakneck Blitz|target:Eevee,player-2,1",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|health:65/115",
            "damage|mon:Eevee,player-2,1|health:57/100",
            "move|mon:Eevee,player-2,1|name:Me First|noanim",
            "fail|mon:Eevee,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn curse_applies_boosts_for_non_ghost_user() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[0].item = Some("Ghostium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 12,zmove"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Pikachu,player-1,1|condition:Z-Power",
            "move|mon:Pikachu,player-1,1|name:Curse|target:Pikachu,player-1,1|zpower",
            "boost|mon:Pikachu,player-1,1|stat:atk|by:1|from:Z-Power",
            "boost|mon:Pikachu,player-1,1|stat:atk|by:1",
            "boost|mon:Pikachu,player-1,1|stat:def|by:1",
            "unboost|mon:Pikachu,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn curse_applies_heal_for_ghost_user() {
    let mut pikachu = pikachu().unwrap();
    pikachu.members[2].item = Some("Ghostium Z".to_owned());
    let mut battle = make_battle(100, BattleType::Singles, pikachu, eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,zmove"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "singleturn|mon:Mimikyu,player-1,1|condition:Z-Power",
            "move|mon:Mimikyu,player-1,1|name:Curse|target:Eevee,player-2,1|zpower",
            "split|side:0",
            "heal|mon:Mimikyu,player-1,1|health:115/115",
            "heal|mon:Mimikyu,player-1,1|health:100/100",
            "start|mon:Eevee,player-2,1|move:Curse",
            "split|side:0",
            "damage|mon:Mimikyu,player-1,1|health:58/115",
            "damage|mon:Mimikyu,player-1,1|health:51/100",
            "split|side:1",
            "damage|mon:Eevee,player-2,1|from:move:Curse|health:87/115",
            "damage|mon:Eevee,player-2,1|from:move:Curse|health:76/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
