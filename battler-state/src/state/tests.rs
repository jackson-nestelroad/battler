#[cfg(test)]
mod state_test {
    use alloc::{
        borrow::ToOwned,
        vec::Vec,
    };

    use hashbrown::{
        HashMap,
        HashSet,
    };

    use crate::{
        log::Log,
        state::{
            BattlePhase,
            BattleState,
            MonBattleAppearanceReference,
            alter_battle_state,
        },
        state_selectors,
        ui,
    };

    fn squirtle_ref() -> MonBattleAppearanceReference {
        MonBattleAppearanceReference {
            player: "player-1".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        }
    }

    fn charmander_ref() -> MonBattleAppearanceReference {
        MonBattleAppearanceReference {
            player: "player-2".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        }
    }

    fn setup_singles_battle(extra_logs: &[&str]) -> BattleState {
        let mut logs = Vec::from_iter([
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
        ]);
        logs.extend_from_slice(extra_logs);
        let log = Log::new(&logs).unwrap();
        alter_battle_state(BattleState::default(), &log).unwrap()
    }

    #[test]
    fn constructs_sides_and_players_before_battle_start() {
        let log = Log::new(&[
            "info|battletype:Singles",
            "info|environment:Normal|time:Evening",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "turn|turn:1",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        assert_eq!(state.phase, BattlePhase::Battle);
        assert_eq!(state.turn, 1);
        assert_eq!(state.field.environment.as_deref(), Some("Normal"));
        assert_eq!(state.field.time.as_deref(), Some("Evening"));
        assert_eq!(state.field.sides[0].name, "Side 1");
        assert_eq!(state.field.sides[1].name, "Side 2");
        assert_eq!(
            state.field.sides[0].players.get("player-1").unwrap().name,
            "Player 1"
        );
        assert_eq!(
            state.field.sides[1].players.get("player-2").unwrap().name,
            "Player 2"
        );
        assert!(state.ui_log.iter().all(|l| l.is_empty()));
    }

    #[test]
    fn adds_mon_for_initial_switch_in() {
        let state = setup_singles_battle(&[]);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        assert_eq!(state.field.sides[0].active[0], Some(sq.clone()));
        assert_eq!(state.field.sides[1].active[0], Some(ch.clone()));
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert_eq!(sq_mon.physical_appearance.name, "Squirtle");
        assert_eq!(sq_mon.physical_appearance.species, "Squirtle");
        assert_eq!(state.ui_log[0].len(), 2);
        assert!(state.ui_log[1].is_empty());
    }

    #[test]
    fn records_simple_move_and_damage() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:75/100",
        ]);
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_health(&state, &ch).unwrap(),
            Some((75, 100))
        );
        let sq = squirtle_ref();
        let moves = state_selectors::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(moves.contains(&"Pound"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Move {
                    name: "Pound".to_owned(),
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }),
                    target: Some(ui::MoveTarget::Single(ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    }))),
                    animate: true,
                    animate_only: false
                },
                ui::UiLogEntry::Damage {
                    health: (75, 100),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "75/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_new_mon_revealed_from_switch() {
        let state = setup_singles_battle(&[
            "switch|player:player-1|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:5|gender:M",
        ]);
        let p1_mons = &state.field.sides[0].players.get("player-1").unwrap().mons;
        assert_eq!(p1_mons.len(), 2);
        assert_eq!(p1_mons[1].physical_appearance.name, "Bulbasaur");
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Switch {
                title: "switch".to_owned(),
                player: "player-1".to_owned(),
                mon: 1,
                into_position: ui::FieldPosition {
                    side: 0,
                    position: 0
                }
            }])
        );
    }

    #[test]
    fn uses_old_mon_reappeared_from_switch() {
        let mut logs = Vec::from_iter([
            "switch|player:player-1|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:5|gender:M",
        ]);
        let state = setup_singles_battle(&logs);
        assert_eq!(
            state.field.sides[0].active[0].as_ref().unwrap().mon_index,
            1
        );

        logs.push("switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M");
        let state = setup_singles_battle(&logs);
        let p1_mons = &state.field.sides[0].players.get("player-1").unwrap().mons;
        assert_eq!(p1_mons.len(), 2);
        assert_eq!(
            state.field.sides[0].active[0].as_ref().unwrap().mon_index,
            0
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Switch {
                    title: "switch".to_owned(),
                    player: "player-1".to_owned(),
                    mon: 1,
                    into_position: ui::FieldPosition {
                        side: 0,
                        position: 0
                    }
                },
                ui::UiLogEntry::Switch {
                    title: "switch".to_owned(),
                    player: "player-1".to_owned(),
                    mon: 0,
                    into_position: ui::FieldPosition {
                        side: 0,
                        position: 0
                    }
                }
            ])
        );
    }

    #[test]
    fn updates_ongoing_state() {
        let mut log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        assert!(
            !state.field.sides[1].players.get("player-2").unwrap().mons[0].battle_appearances[0]
                .primary()
                .moves
                .known()
                .contains("Scratch")
        );
        log.extend([
            "move|mon:Charmander,player-2,1|name:Scratch|target:Squirtle,player-1,1",
            "damage|mon:Charmander,player-2,1|health:80/100",
        ])
        .unwrap();
        let state = alter_battle_state(state, &log).unwrap();
        assert!(
            state.field.sides[1].players.get("player-2").unwrap().mons[0].battle_appearances[0]
                .primary()
                .moves
                .known()
                .contains("Scratch")
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Move {
                    name: "Scratch".to_owned(),
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    }),
                    target: Some(ui::MoveTarget::Single(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }))),
                    animate: true,
                    animate_only: false
                },
                ui::UiLogEntry::Damage {
                    health: (80, 100),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "80/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_fainted_mon() {
        let state = setup_singles_battle(&[
            "damage|mon:Charmander,player-2,1|health:0",
            "faint|mon:Charmander,player-2,1",
        ]);
        let ch = charmander_ref();
        let ch_mon = state.field.mon_by_reference_or_else(&ch).unwrap();
        assert!(ch_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (0, 1),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([("health".to_owned(), "0".to_owned())]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Faint {
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn keeps_track_of_multiple_battle_appearances_due_to_single_illusion_user_with_unique_level() {
        let mut log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "residual",
            "turn|turn:2",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:6|gender:M",
            "residual",
            "turn|turn:3",
        ])
        .unwrap();

        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        let p2 = &state.field.sides[1].players["player-2"];
        assert_eq!(p2.mons.len(), 3);
        assert_eq!(p2.mons[0].physical_appearance.name, "Charmander");
        assert_eq!(
            p2.mons[0].battle_appearances[0].primary().level.known(),
            Some(&5)
        );
        assert_eq!(p2.mons[1].physical_appearance.name, "Bulbasaur");
        assert_eq!(
            p2.mons[1].battle_appearances[0].primary().level.known(),
            Some(&5)
        );
        assert_eq!(p2.mons[2].physical_appearance.name, "Charmander");
        assert_eq!(
            p2.mons[2].battle_appearances[0].primary().level.known(),
            Some(&6)
        );

        log.extend([
            "damage|mon:Charmander,player-2,1|health:75/100",
            "replace|player:player-2|position:1|name:Zoroark|health:75/100|species:Zoroark|level:6|gender:F",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "residual",
            "turn|turn:4",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        assert_eq!(
            state.ui_log[3],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (75, 100),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "75/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Switch {
                    title: "replace".to_owned(),
                    player: "player-2".to_owned(),
                    mon: 2,
                    into_position: ui::FieldPosition {
                        side: 1,
                        position: 0
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "end".to_owned(),
                    effect: ui::EffectData {
                        effect: Some(ui::Effect {
                            effect_type: Some("ability".to_owned()),
                            name: "Illusion".to_owned()
                        }),
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
        let p2 = &state.field.sides[1].players["player-2"];
        assert_eq!(p2.mons.len(), 3);
        assert_eq!(p2.mons[2].physical_appearance.name, "Zoroark");
        assert_eq!(
            p2.mons[2].battle_appearances[0].primary().level.known(),
            Some(&6)
        );
        assert_eq!(
            p2.mons[2].battle_appearances[0]
                .primary()
                .ability
                .known()
                .map(|s| s.as_str()),
            Some("Illusion")
        );

        log.extend([
            "move|mon:Zoroark,player-2,1|name:Bite",
            "turn|turn:5",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "move|mon:Bulbasaur,player-2,1|name:Absorb",
            "turn|turn:6",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:7",
            "move|mon:Charmander,player-2,1|name:Growl",
            "turn|turn:8",
            "switch|player:player-2|position:1|name:Bulbasaur|health:75/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:9",
            "move|mon:Bulbasaur,player-2,1|name:Dark Pulse",
            "turn|turn:10",
            "damage|mon:Bulbasaur,player-2,1|health:50/100",
            "replace|player:player-2|position:1|name:Zoroark|health:50/100|species:Zoroark|level:6|gender:F",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "turn|turn:11",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        assert_eq!(
            state.ui_log[10],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (50, 100),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "50/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Switch {
                    title: "replace".to_owned(),
                    player: "player-2".to_owned(),
                    mon: 2,
                    into_position: ui::FieldPosition {
                        side: 1,
                        position: 0
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "end".to_owned(),
                    effect: ui::EffectData {
                        effect: Some(ui::Effect {
                            effect_type: Some("ability".to_owned()),
                            name: "Illusion".to_owned()
                        }),
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
        let p2 = &state.field.sides[1].players["player-2"];
        assert_eq!(
            p2.mons[0].battle_appearances[0]
                .primary()
                .moves
                .known()
                .iter()
                .collect::<Vec<_>>(),
            Vec::from_iter(["Growl"])
        );
        assert_eq!(
            p2.mons[1].battle_appearances[0]
                .primary()
                .moves
                .known()
                .iter()
                .collect::<Vec<_>>(),
            Vec::from_iter(["Absorb"])
        );
        let zoroark_moves = p2.mons[2].battle_appearances[0]
            .primary()
            .moves
            .known()
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        assert!(zoroark_moves.contains("Bite"));
        assert!(zoroark_moves.contains("Dark Pulse"));

        log.extend([
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:12",
            "switch|player:player-2|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:13",
            "move|mon:Bulbasaur,player-2,1|name:Crunch",
            "turn|turn:14",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:15",
            "switch|player:player-2|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:16",
            "damage|mon:Bulbasaur,player-2,1|health:25/100",
            "replace|player:player-2|position:1|name:Zoroark|health:25/100|species:Zoroark|level:6|gender:F",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "turn|turn:17",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        assert_eq!(
            state.ui_log[16],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (25, 100),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "25/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Switch {
                    title: "replace".to_owned(),
                    player: "player-2".to_owned(),
                    mon: 2,
                    into_position: ui::FieldPosition {
                        side: 1,
                        position: 0
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "end".to_owned(),
                    effect: ui::EffectData {
                        effect: Some(ui::Effect {
                            effect_type: Some("ability".to_owned()),
                            name: "Illusion".to_owned()
                        }),
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
        let p2 = &state.field.sides[1].players["player-2"];
        assert_eq!(p2.mons.len(), 3);
        assert_eq!(p2.mons[2].physical_appearance.name, "Zoroark");
        let bulbasaur_moves = p2.mons[1].battle_appearances[1]
            .primary()
            .moves
            .known()
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        assert!(bulbasaur_moves.contains("Crunch"));
        let zoroark_moves_final = p2.mons[2].battle_appearances[0]
            .primary()
            .moves
            .known()
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        assert!(!zoroark_moves_final.contains("Crunch"));
        assert!(zoroark_moves_final.contains("Bite"));
        assert!(zoroark_moves_final.contains("Dark Pulse"));
    }

    #[test]
    fn keeps_track_of_multiple_battle_appearances_due_to_single_illusion_user_with_same_level() {
        let log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "residual",
            "turn|turn:2",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "residual",
            "turn|turn:3",
            "damage|mon:Charmander,player-2,1|health:75/100",
            "replace|player:player-2|position:1|name:Zoroark|health:75/100|species:Zoroark|level:5|gender:F",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "residual",
            "turn|turn:4",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        let p2 = &state.field.sides[1].players["player-2"];
        assert_eq!(p2.mons.len(), 3);
        assert_eq!(p2.mons[0].physical_appearance.name, "Charmander");
        assert_eq!(p2.mons[1].physical_appearance.name, "Bulbasaur");
        assert_eq!(p2.mons[2].physical_appearance.name, "Zoroark");
        assert_eq!(
            state.ui_log[3],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (75, 100),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "75/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Switch {
                    title: "replace".to_owned(),
                    player: "player-2".to_owned(),
                    mon: 2,
                    into_position: ui::FieldPosition {
                        side: 1,
                        position: 0
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "end".to_owned(),
                    effect: ui::EffectData {
                        effect: Some(ui::Effect {
                            effect_type: Some("ability".to_owned()),
                            name: "Illusion".to_owned()
                        }),
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn illusion_user_faints_before_being_revealed() {
        let state = setup_singles_battle(&[
            "damage|mon:Charmander,player-2,1|health:0",
            "faint|mon:Charmander,player-2,1",
        ]);
        let p2 = &state.field.sides[1].players["player-2"];
        assert!(p2.mons[0].fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (0, 1),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([("health".to_owned(), "0".to_owned())]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Faint {
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn corrects_fainted_illusion_user_with_multiple_illusion_users() {
        let state = setup_singles_battle(&[
            "damage|mon:Charmander,player-2,1|health:0",
            "faint|mon:Charmander,player-2,1",
        ]);
        assert!(state.field.sides[1].players["player-2"].mons[0].fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (0, 1),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([("health".to_owned(), "0".to_owned())]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Faint {
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_ability_from_source_effect() {
        let state = setup_singles_battle(&[
            "ability|mon:Squirtle,player-1,1|ability:Drizzle|from:ability:Drizzle|of:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &sq).unwrap(),
            Some("Drizzle")
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "ability".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("ability".to_owned()),
                        name: "Drizzle".to_owned()
                    }),
                    source_effect: Some(ui::Effect {
                        effect_type: Some("ability".to_owned()),
                        name: "Drizzle".to_owned()
                    }),
                    source: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_item_from_source_effect() {
        let state = setup_singles_battle(&[
            "item|mon:Squirtle,player-1,1|item:Leftovers|from:item:Leftovers|of:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_item(&state, &sq).unwrap(),
            Some("Leftovers")
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "item".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("item".to_owned()),
                        name: "Leftovers".to_owned()
                    }),
                    source_effect: Some(ui::Effect {
                        effect_type: Some("item".to_owned()),
                        name: "Leftovers".to_owned()
                    }),
                    source: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_ability() {
        let state = setup_singles_battle(&["ability|mon:Charmander,player-2,1|ability:Blaze"]);
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &ch).unwrap(),
            Some("Blaze")
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "ability".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("ability".to_owned()),
                        name: "Blaze".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_volatile_ability() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Skill Swap|target:Charmander,player-2,1",
            "activate|mon:Charmander,player-2,1|move:Skill Swap|of:Squirtle,player-1,1",
            "abilityend|mon:Squirtle,player-1,1|ability:Torrent|from:move:Skill Swap",
            "ability|mon:Squirtle,player-1,1|ability:Blaze|from:move:Skill Swap",
            "abilityend|mon:Charmander,player-2,1|ability:Blaze|from:move:Skill Swap|of:Squirtle,player-1,1",
            "ability|mon:Charmander,player-2,1|ability:Torrent|from:move:Skill Swap|of:Squirtle,player-1,1",
        ]);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        let ch_mon = state.field.mon_by_reference_or_else(&ch).unwrap();
        assert_eq!(sq_mon.volatile_data.ability.as_deref(), Some("Blaze"));
        assert_eq!(ch_mon.volatile_data.ability.as_deref(), Some("Torrent"));
        assert_eq!(
            state.ui_log[1][3],
            ui::UiLogEntry::Effect {
                title: "ability".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("ability".to_owned()),
                        name: "Blaze".to_owned()
                    }),
                    source_effect: Some(ui::Effect {
                        effect_type: Some("move".to_owned()),
                        name: "Skill Swap".to_owned()
                    }),
                    ..Default::default()
                }
            }
        );
    }

    #[test]
    fn records_ability_from_activation() {
        let state = setup_singles_battle(&["activate|mon:Squirtle,player-1,1|ability:Intimidate"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &sq).unwrap(),
            Some("Intimidate")
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "activate".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("ability".to_owned()),
                        name: "Intimidate".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_item_from_activation() {
        let state = setup_singles_battle(&["activate|mon:Squirtle,player-1,1|item:Quick Claw"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_item(&state, &sq).unwrap(),
            Some("Quick Claw")
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "activate".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("item".to_owned()),
                        name: "Quick Claw".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn does_not_record_item_after_item_end_log() {
        let state = setup_singles_battle(&[
            "item|mon:Squirtle,player-1,1|item:Leftovers",
            "itemend|mon:Squirtle,player-1,1|item:Leftovers",
        ]);
        let sq = squirtle_ref();
        assert_eq!(state_selectors::mon_item(&state, &sq).unwrap(), None);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "item".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("item".to_owned()),
                            name: "Leftovers".to_owned()
                        }),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "itemend".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("item".to_owned()),
                            name: "Leftovers".to_owned()
                        }),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_and_switches_out_caught_mon() {
        let state = setup_singles_battle(&[
            "catch|player:player-1|mon:Charmander,player-2,1|item:Ultra Ball|shakes:4",
        ]);
        assert!(state.field.sides[1].active[0].is_some());
        assert!(state.field.sides[1].players["player-2"].mons[0].fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Caught {
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("item".to_owned()),
                        name: "Ultra Ball".to_owned()
                    }),
                    player: Some("player-1".to_owned()),
                    additional: HashMap::from_iter([("shakes".to_owned(), "4".to_owned())]),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_stat_boosts() {
        let mut logs = Vec::from_iter(["boost|mon:Squirtle,player-1,1|stat:atk|by:2"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);
        assert_eq!(boosts.get(battler::Boost::Def), 0);

        logs.extend_from_slice(&[
            "turn|turn:2",
            "boost|mon:Squirtle,player-1,1|stat:def|by:-1",
        ]);
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);
        assert_eq!(boosts.get(battler::Boost::Def), -1);

        logs.extend_from_slice(&["turn|turn:3", "clearallboosts", "turn|turn:4"]);
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 0);
        assert_eq!(boosts.get(battler::Boost::Def), 0);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::StatBoost {
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
                stat: "atk".to_owned(),
                by: 2
            }])
        );
        assert_eq!(
            state.ui_log[2],
            Vec::from_iter([ui::UiLogEntry::StatBoost {
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
                stat: "def".to_owned(),
                by: -1
            }])
        );
        assert_eq!(
            state.ui_log[3],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "clearallboosts".to_owned(),
                effect: ui::EffectData {
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_weather() {
        let mut logs = Vec::from_iter(["weather|weather:Rain"]);
        let state = setup_singles_battle(&logs);
        assert_eq!(state_selectors::field_weather(&state), Some("Rain"));

        logs.extend_from_slice(&["turn|turn:2", "clearweather", "turn|turn:3"]);
        let state = setup_singles_battle(&logs);
        assert_eq!(state_selectors::field_weather(&state), None);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "weather".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("weather".to_owned()),
                        name: "Rain".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
        assert_eq!(
            state.ui_log[2],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "clearweather".to_owned(),
                effect: ui::EffectData {
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_status() {
        let mut logs = Vec::from_iter(["status|mon:Squirtle,player-1,1|status:Paralysis"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_status(&state, &sq).unwrap(),
            Some("Paralysis")
        );

        logs.extend_from_slice(&[
            "turn|turn:2",
            "curestatus|mon:Squirtle,player-1,1|status:Paralysis",
            "turn|turn:3",
        ]);
        let state = setup_singles_battle(&logs);
        assert_eq!(state_selectors::mon_status(&state, &sq).unwrap(), None);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "status".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("status".to_owned()),
                        name: "Paralysis".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
        assert_eq!(
            state.ui_log[2],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "curestatus".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("status".to_owned()),
                        name: "Paralysis".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_health_changes() {
        let mut logs = Vec::from_iter(["damage|mon:Squirtle,player-1,1|health:50/100"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_health(&state, &sq).unwrap(),
            Some((50, 100))
        );

        logs.push("heal|mon:Squirtle,player-1,1|health:75/100");
        let state = setup_singles_battle(&logs);
        assert_eq!(
            state_selectors::mon_health(&state, &sq).unwrap(),
            Some((75, 100))
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (50, 100),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "50/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Heal {
                    health: (75, 100),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "75/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_volatile_condition() {
        let mut logs = Vec::from_iter(["start|mon:Squirtle,player-1,1|volatile:Substitute"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let conds = state_selectors::mon_conditions(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(conds.contains(&"Substitute"));

        logs.extend_from_slice(&[
            "turn|turn:2",
            "end|mon:Squirtle,player-1,1|volatile:Substitute",
            "turn|turn:3",
        ]);
        let state = setup_singles_battle(&logs);
        let conds = state_selectors::mon_conditions(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!conds.contains(&"Substitute"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "start".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("volatile".to_owned()),
                        name: "Substitute".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
        assert_eq!(
            state.ui_log[2],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "end".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("volatile".to_owned()),
                        name: "Substitute".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_field_condition() {
        let mut logs = Vec::from_iter(["fieldstart|condition:Trick Room"]);
        let state = setup_singles_battle(&logs);
        let conds = state_selectors::field_conditions(&state).collect::<Vec<_>>();
        assert!(conds.contains(&"Trick Room"));

        logs.extend_from_slice(&[
            "turn|turn:2",
            "fieldend|condition:Trick Room",
            "turn|turn:3",
        ]);
        let state = setup_singles_battle(&logs);
        let conds = state_selectors::field_conditions(&state).collect::<Vec<_>>();
        assert!(!conds.contains(&"Trick Room"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "fieldstart".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("condition".to_owned()),
                        name: "Trick Room".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
        assert_eq!(
            state.ui_log[2],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "fieldend".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("condition".to_owned()),
                        name: "Trick Room".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_forme_change() {
        let state =
            setup_singles_battle(&["formechange|mon:Squirtle,player-1,1|species:Squirtle-Mega"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_species(&state, &sq).unwrap(),
            "Squirtle-Mega"
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "formechange".to_owned(),
                species: "Squirtle-Mega".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("species".to_owned()),
                        name: "Squirtle-Mega".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_item_changes() {
        let mut logs = Vec::from_iter(["item|mon:Squirtle,player-1,1|item:Leftovers"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_item(&state, &sq).unwrap(),
            Some("Leftovers")
        );

        logs.push("itemend|mon:Squirtle,player-1,1|item:Leftovers");
        let state = setup_singles_battle(&logs);
        assert_eq!(state_selectors::mon_item(&state, &sq).unwrap(), None);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "item".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("item".to_owned()),
                            name: "Leftovers".to_owned()
                        }),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "itemend".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("item".to_owned()),
                            name: "Leftovers".to_owned()
                        }),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_move_volatile_with_prepare() {
        let state = setup_singles_battle(&["prepare|mon:Squirtle,player-1,1|move:Solar Beam"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Solar Beam"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "prepare".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("move".to_owned()),
                        name: "Solar Beam".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_move_volatile_until_next_move() {
        let state = setup_singles_battle(&["singlemove|mon:Squirtle,player-1,1|move:Destiny Bond"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Destiny Bond"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "singlemove".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("move".to_owned()),
                        name: "Destiny Bond".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn does_not_record_externally_used_move() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Metronome|target:Charmander,player-2,1",
            "move|mon:Squirtle,player-1,1|name:Ice Beam|target:Charmander,player-2,1|from:move:Metronome",
        ]);
        let sq = squirtle_ref();
        let moves = state_selectors::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!moves.contains(&"Ice Beam"));
        assert_eq!(state.ui_log[1].len(), 2);
    }

    #[test]
    fn records_side_condition() {
        let mut logs = Vec::from_iter(["sidestart|side:0|condition:Spikes"]);
        let state = setup_singles_battle(&logs);
        let conds = state_selectors::side_conditions(&state, 0)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(conds.contains(&"Spikes"));

        logs.extend_from_slice(&[
            "turn|turn:2",
            "sideend|side:0|condition:Spikes",
            "turn|turn:3",
        ]);
        let state = setup_singles_battle(&logs);
        let conds = state_selectors::side_conditions(&state, 0)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!conds.contains(&"Spikes"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "sidestart".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("condition".to_owned()),
                        name: "Spikes".to_owned()
                    }),
                    side: Some(0),
                    ..Default::default()
                }
            }])
        );
        assert_eq!(
            state.ui_log[2],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "sideend".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("condition".to_owned()),
                        name: "Spikes".to_owned()
                    }),
                    side: Some(0),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_transformation() {
        let state = setup_singles_battle(&[
            "transform|mon:Squirtle,player-1,1|species:Charmander|into:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.transformed.is_some());
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "transform".to_owned(),
                species: "Charmander".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("species".to_owned()),
                        name: "Charmander".to_owned()
                    }),
                    additional: HashMap::from_iter([(
                        "into".to_owned(),
                        "Charmander,player-2,1".to_owned()
                    )]),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_type_change() {
        let state = setup_singles_battle(&["typechange|mon:Squirtle,player-1,1|types:Fire/Flying"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert_eq!(
            sq_mon.volatile_data.types,
            Vec::from_iter(["Fire", "Flying"])
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "typechange".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    additional: HashMap::from_iter([(
                        "types".to_owned(),
                        "Fire/Flying".to_owned()
                    )]),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_escape() {
        let mut log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "cannotescape|player:player-1",
            "turn|turn:2",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        assert_eq!(state.phase, BattlePhase::Battle);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::CannotEscape {
                player: "player-1".to_owned()
            }])
        );

        log.extend(["escaped|player:player-1"]).unwrap();
        let state = alter_battle_state(state, &log).unwrap();
        assert_eq!(
            state.ui_log[2],
            Vec::from_iter([ui::UiLogEntry::Leave {
                title: "escaped".to_owned(),
                player: "player-1".to_owned(),
                positions: HashSet::from_iter([ui::FieldPosition {
                    side: 0,
                    position: 0
                }])
            }])
        );
    }

    #[test]
    fn records_forfeit() {
        let state = setup_singles_battle(&["forfeited|player:player-1"]);
        assert_eq!(state.phase, BattlePhase::Battle);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Leave {
                title: "forfeited".to_owned(),
                player: "player-1".to_owned(),
                positions: HashSet::from_iter([ui::FieldPosition {
                    side: 0,
                    position: 0
                }])
            }])
        );
    }

    #[test]
    fn records_learned_move() {
        let mut logs = Vec::from_iter(["move|mon:Squirtle,player-1,1|name:Pound"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let moves = state_selectors::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(moves.contains(&"Pound"));
        assert!(!moves.contains(&"Water Gun"));

        logs.extend_from_slice(&[
            "turn|turn:2",
            "learnedmove|mon:Squirtle,player-1,1|move:Water Gun|forgot:Pound",
        ]);
        let state = setup_singles_battle(&logs);
        let moves = state_selectors::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(moves.contains(&"Water Gun"));
        assert!(!moves.contains(&"Pound"));
        assert_eq!(
            state.ui_log[2],
            Vec::from_iter([ui::UiLogEntry::MoveUpdate {
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
                move_name: "Water Gun".to_owned(),
                learned: true,
                forgot: Some("Pound".to_owned()),
            }])
        );
    }

    #[test]
    fn records_multihit_move() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Double Slap|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:90/100",
            "damage|mon:Charmander,player-2,1|health:80/100",
            "hitcount|mon:Charmander,player-2,1|count:2",
        ]);
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_health(&state, &ch).unwrap(),
            Some((80, 100))
        );
        assert_eq!(
            state.ui_log[1][3],
            ui::UiLogEntry::Effect {
                title: "hitcount".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    })),
                    additional: HashMap::from_iter([("count".to_owned(), "2".to_owned())]),
                    ..Default::default()
                }
            }
        );
    }

    #[test]
    fn records_tie() {
        let state = setup_singles_battle(&["tie"]);
        assert_eq!(state.phase, BattlePhase::Finished);
        assert_eq!(state.winning_side, None);
        assert_eq!(state.ui_log[1], Vec::from_iter([ui::UiLogEntry::Tie]));
    }

    #[test]
    fn records_win() {
        let state = setup_singles_battle(&["win|side:0"]);
        assert_eq!(state.phase, BattlePhase::Finished);
        assert_eq!(state.winning_side, Some(0));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Win { side: 0 }])
        );
    }

    #[test]
    fn records_use_item() {
        let state = setup_singles_battle(&[
            "useitem|player:player-1|name:Oran Berry|target:Squirtle,player-1,1",
        ]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UseItem {
                player: "player-1".to_owned(),
                item: "Oran Berry".to_owned(),
                target: Some(ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }))
            }])
        );
    }

    #[test]
    fn records_copied_boosts() {
        let mut logs = Vec::from_iter(["boost|mon:Charmander,player-2,1|stat:atk|by:2"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Atk),
            0
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Atk),
            2
        );

        logs.push("copyboosts|mon:Squirtle,player-1,1|of:Charmander,player-2,1");
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    }),
                    stat: "atk".to_owned(),
                    by: 2
                },
                ui::UiLogEntry::Effect {
                    title: "copyboosts".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        source: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_swapped_boosts_for_all_stats() {
        let mut logs = Vec::from_iter([
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
            "boost|mon:Charmander,player-2,1|stat:def|by:1",
        ]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Atk),
            2
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Def),
            0
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Def),
            1
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Atk),
            0
        );

        logs.push("swapboosts|mon:Squirtle,player-1,1|of:Charmander,player-2,1");
        let state = setup_singles_battle(&logs);
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Atk),
            0
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Def),
            1
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Atk),
            2
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Def),
            0
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }),
                    stat: "atk".to_owned(),
                    by: 2
                },
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    }),
                    stat: "def".to_owned(),
                    by: 1
                },
                ui::UiLogEntry::Effect {
                    title: "swapboosts".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        source: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_swapped_boosts_for_some_stats() {
        let mut logs = Vec::from_iter([
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
            "boost|mon:Charmander,player-2,1|stat:def|by:1",
        ]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Atk),
            2
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Def),
            0
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Def),
            1
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Atk),
            0
        );

        logs.push("swapboosts|mon:Squirtle,player-1,1|of:Charmander,player-2,1|stats:atk");
        let state = setup_singles_battle(&logs);
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Atk),
            0
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Def),
            0
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Atk),
            2
        );
        assert_eq!(
            state_selectors::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Def),
            1
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }),
                    stat: "atk".to_owned(),
                    by: 2
                },
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    }),
                    stat: "def".to_owned(),
                    by: 1
                },
                ui::UiLogEntry::Effect {
                    title: "swapboosts".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        source: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 1,
                            position: 0
                        })),
                        additional: HashMap::from_iter([("stats".to_owned(), "atk".to_owned())]),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_mega_evolution() {
        let state = setup_singles_battle(&[
            "mega|mon:Squirtle,player-1,1|species:Squirtle-Mega|item:Squirtlite",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_species(&state, &sq).unwrap(),
            "Squirtle-Mega"
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "mega".to_owned(),
                species: "Squirtle-Mega".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("item".to_owned()),
                        name: "Squirtlite".to_owned()
                    }),
                    additional: HashMap::from_iter([(
                        "species".to_owned(),
                        "Squirtle-Mega".to_owned()
                    )]),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_dynamax() {
        let state = setup_singles_battle(&["dynamax|mon:Squirtle,player-1,1"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Dynamax"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "dynamax".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_gigantamax() {
        let state =
            setup_singles_battle(&["gigantamax|mon:Squirtle,player-1,1|species:Squirtle-Gmax"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_species(&state, &sq).unwrap(),
            "Squirtle-Gmax"
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "gigantamax".to_owned(),
                species: "Squirtle-Gmax".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("species".to_owned()),
                        name: "Squirtle-Gmax".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_terastallization() {
        let state = setup_singles_battle(&["tera|mon:Squirtle,player-1,1|type:Fire"]);
        let sq = squirtle_ref();
        let sq_app = state_selectors::mon_battle_appearance_or_else(&state, &sq).unwrap();
        assert_eq!(
            sq_app.terastallization.known().map(|s| s.as_str()),
            Some("Fire")
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "tera".to_owned(),
                effect: ui::EffectData {
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    effect: Some(ui::Effect {
                        effect_type: Some("type".to_owned()),
                        name: "Fire".to_owned()
                    }),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_extension_log() {
        let log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "-battlerservice:timer|battle|remainingsecs:5",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Extension {
                source: "-battlerservice".to_owned(),
                title: "timer".to_owned(),
                values: HashMap::from_iter([
                    ("battle".to_owned(), "".to_owned()),
                    ("remainingsecs".to_owned(), "5".to_owned()),
                ])
            }])
        );
    }

    #[test]
    fn records_additional_state_mutations() {
        let mut logs = Vec::from_iter([
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
            "boost|mon:Squirtle,player-1,1|stat:def|by:-1",
        ]);
        let state = setup_singles_battle(&logs);
        let squirtle_ref = squirtle_ref();
        let boosts = state_selectors::mon_boosts(&state, &squirtle_ref).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);
        assert_eq!(boosts.get(battler::Boost::Def), -1);

        logs.push("invertboosts|mon:Squirtle,player-1,1");
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &squirtle_ref).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), -2);
        assert_eq!(boosts.get(battler::Boost::Def), 1);

        logs.push("clearpositiveboosts|mon:Squirtle,player-1,1");
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &squirtle_ref).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), -2);
        assert_eq!(boosts.get(battler::Boost::Def), 0);

        logs.extend_from_slice(&[
            "addedtype|mon:Squirtle,player-1,1|type:Grass",
            "swapplayer|player:player-1|position:2",
            "turn|turn:2",
        ]);
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &squirtle_ref).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), -2);
        assert_eq!(boosts.get(battler::Boost::Def), 0);

        let data = battler_test_utils::static_local_data_store();
        let types = state_selectors::mon_types(&state, &squirtle_ref, data).unwrap();
        assert!(types.contains(&battler::Type::Water));
        assert!(types.contains(&battler::Type::Grass));

        assert_eq!(
            state.field.sides[0]
                .players
                .get("player-1")
                .unwrap()
                .position,
            2
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }),
                    stat: "atk".to_owned(),
                    by: 2
                },
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }),
                    stat: "def".to_owned(),
                    by: -1
                },
                ui::UiLogEntry::Effect {
                    title: "invertboosts".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "clearpositiveboosts".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "addedtype".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("type".to_owned()),
                            name: "Grass".to_owned()
                        }),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_field_activate() {
        let state = setup_singles_battle(&["fieldactivate|effect:Gravity"]);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "fieldactivate".to_owned(),
                effect: ui::EffectData {
                    effect: None,
                    additional: HashMap::from_iter([("effect".to_owned(), "Gravity".to_owned())]),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_clear_boosts() {
        let mut logs = Vec::from_iter(["boost|mon:Squirtle,player-1,1|stat:atk|by:2"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);

        logs.push("clearboosts|mon:Squirtle,player-1,1");
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 0);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }),
                    stat: "atk".to_owned(),
                    by: 2
                },
                ui::UiLogEntry::Effect {
                    title: "clearboosts".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_clear_negative_boosts() {
        let mut logs = Vec::from_iter([
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
            "boost|mon:Squirtle,player-1,1|stat:def|by:-2",
        ]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);
        assert_eq!(boosts.get(battler::Boost::Def), -2);

        logs.push("clearnegativeboosts|mon:Squirtle,player-1,1");
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);
        assert_eq!(boosts.get(battler::Boost::Def), 0);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }),
                    stat: "atk".to_owned(),
                    by: 2
                },
                ui::UiLogEntry::StatBoost {
                    mon: ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    }),
                    stat: "def".to_owned(),
                    by: -2
                },
                ui::UiLogEntry::Effect {
                    title: "clearnegativeboosts".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_clear_weather() {
        let mut logs = Vec::from_iter(["weather|weather:RainDance"]);
        let state = setup_singles_battle(&logs);
        assert_eq!(state_selectors::field_weather(&state), Some("RainDance"));

        logs.push("clearweather");
        let state = setup_singles_battle(&logs);
        assert_eq!(state_selectors::field_weather(&state), None);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "weather".to_owned(),
                    effect: ui::EffectData {
                        effect: Some(ui::Effect {
                            effect_type: Some("weather".to_owned()),
                            name: "RainDance".to_owned()
                        }),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "clearweather".to_owned(),
                    effect: ui::EffectData {
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_revive() {
        let mut logs = Vec::from_iter([
            "damage|mon:Squirtle,player-1,1|health:0",
            "faint|mon:Squirtle,player-1,1",
        ]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.fainted);

        logs.push("revive|mon:Squirtle,player-1,1|health:50/100");
        let state = setup_singles_battle(&logs);
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state_selectors::mon_health(&state, &sq).unwrap(),
            Some((50, 100))
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Damage {
                    health: (0, 1),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        additional: HashMap::from_iter([("health".to_owned(), "0".to_owned())]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Faint {
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Revive {
                    effect: ui::EffectData {
                        effect: None,
                        player: None,
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "health".to_owned(),
                            "50/100".to_owned()
                        )]),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_set_hp() {
        let state = setup_singles_battle(&["sethp|mon:Squirtle,player-1,1|health:42/100"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_health(&state, &sq).unwrap(),
            Some((42, 100))
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::SetHealth {
                health: (42, 100),
                effect: ui::EffectData {
                    effect: None,
                    player: None,
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    additional: HashMap::from_iter([("health".to_owned(), "42/100".to_owned())]),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_primal_reversion() {
        let state =
            setup_singles_battle(&["primal|mon:Squirtle,player-1,1|species:Squirtle-Primal"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_species(&state, &sq).unwrap(),
            "Squirtle-Primal"
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "primal".to_owned(),
                species: "Squirtle-Primal".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("species".to_owned()),
                        name: "Squirtle-Primal".to_owned(),
                    }),
                    player: None,
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_ultra_burst() {
        let state = setup_singles_battle(&["ultra|mon:Squirtle,player-1,1|species:Squirtle-Ultra"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_species(&state, &sq).unwrap(),
            "Squirtle-Ultra"
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "ultra".to_owned(),
                species: "Squirtle-Ultra".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("species".to_owned()),
                        name: "Squirtle-Ultra".to_owned(),
                    }),
                    player: None,
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_dynamax_reversion() {
        let mut logs = Vec::from_iter(["dynamax|mon:Squirtle,player-1,1"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Dynamax"));

        logs.push("revertdynamax|mon:Squirtle,player-1,1");
        let state = setup_singles_battle(&logs);
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.volatile_data.conditions.contains_key("Dynamax"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "dynamax".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "revertdynamax".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_gigantamax_reversion() {
        let state =
            setup_singles_battle(&["revertgigantamax|mon:Squirtle,player-1,1|species:Squirtle"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_species(&state, &sq).unwrap(),
            "Squirtle"
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "revertgigantamax".to_owned(),
                species: "Squirtle".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("species".to_owned()),
                        name: "Squirtle".to_owned(),
                    }),
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_mega_reversion() {
        let state = setup_singles_battle(&["revertmega|mon:Squirtle,player-1,1|species:Squirtle"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_species(&state, &sq).unwrap(),
            "Squirtle"
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "revertmega".to_owned(),
                species: "Squirtle".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("species".to_owned()),
                        name: "Squirtle".to_owned(),
                    }),
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_tera_reversion() {
        let mut logs = Vec::from_iter(["tera|mon:Squirtle,player-1,1|type:Fire"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let sq_app = state_selectors::mon_battle_appearance_or_else(&state, &sq).unwrap();
        assert_eq!(
            sq_app.terastallization.known().map(|s| s.as_str()),
            Some("Fire")
        );

        logs.push("reverttera|mon:Squirtle,player-1,1");
        let state = setup_singles_battle(&logs);
        let sq_app = state_selectors::mon_battle_appearance_or_else(&state, &sq).unwrap();
        assert_eq!(
            sq_app.terastallization.known().map(|s| s.as_str()),
            Some("")
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "tera".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("type".to_owned()),
                            name: "Fire".to_owned()
                        }),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "reverttera".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_species_change() {
        let state = setup_singles_battle(&[
            "specieschange|player:player-1|position:1|name:Squirtle|health:100/100|species:Wartortle|level:5|gender:M",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_species(&state, &sq).unwrap(),
            "Wartortle"
        );
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::UpdateAppearance {
                title: "specieschange".to_owned(),
                species: "Wartortle".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("species".to_owned()),
                        name: "Wartortle".to_owned(),
                    }),
                    player: Some("player-1".to_owned()),
                    target: None,
                    additional: HashMap::from_iter([
                        ("position".to_owned(), "1".to_owned()),
                        ("gender".to_owned(), "M".to_owned()),
                        ("name".to_owned(), "Squirtle".to_owned()),
                        ("health".to_owned(), "100/100".to_owned()),
                        ("level".to_owned(), "5".to_owned()),
                    ]),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_reset_type_change() {
        let mut logs = Vec::from_iter(["typechange|mon:Squirtle,player-1,1|types:Fire/Flying"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert_eq!(
            sq_mon.volatile_data.types,
            Vec::from_iter(["Fire".to_owned(), "Flying".to_owned()])
        );

        logs.push("resettypechange|mon:Squirtle,player-1,1");
        let state = setup_singles_battle(&logs);
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.types.is_empty());
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "typechange".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "types".to_owned(),
                            "Fire/Flying".to_owned()
                        )]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "resettypechange".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_active_position_swap() {
        let log = Log::new(&[
            "info|battletype:Doubles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:2",
            "teamsize|player:player-2|size:2",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-1|position:2|name:Wartortle|health:100/100|species:Wartortle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "switch|player:player-2|position:2|name:Charmeleon|health:100/100|species:Charmeleon|level:5|gender:M",
            "turn|turn:1",
            "swap|mon:Squirtle,player-1,1|position:2",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        let side = &state.field.sides[0];
        assert_eq!(
            side.active[0].as_ref().unwrap().mon_index,
            1 // Wartortle
        );
        assert_eq!(
            side.active[1].as_ref().unwrap().mon_index,
            0 // Squirtle
        );
        assert!(state.ui_log[1].is_empty());
    }

    #[test]
    fn records_swap_side_conditions() {
        let state = setup_singles_battle(&[
            "sidestart|side:0|condition:Spikes",
            "swapsideconditions|side:0|with:1",
        ]);
        let side0_conds = state_selectors::side_conditions(&state, 0)
            .unwrap()
            .collect::<Vec<_>>();
        let side1_conds = state_selectors::side_conditions(&state, 1)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(side0_conds.is_empty());
        assert!(side1_conds.contains(&"Spikes"));
        assert_eq!(state.ui_log[1].len(), 1); // Only the sidestart UI log entry is pushed.
    }

    #[test]
    fn records_swap_single_side_condition() {
        let state = setup_singles_battle(&[
            "sidestart|side:0|condition:Spikes",
            "swapsidecondition|side:1|source:0|condition:Spikes",
        ]);
        let side0_conds = state_selectors::side_conditions(&state, 0)
            .unwrap()
            .collect::<Vec<_>>();
        let side1_conds = state_selectors::side_conditions(&state, 1)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(side0_conds.is_empty());
        assert!(side1_conds.contains(&"Spikes"));
        assert_eq!(state.ui_log[1].len(), 1); // Only the sidestart UI log entry is pushed.
    }

    #[test]
    fn records_single_move_volatile() {
        let state = setup_singles_battle(&["singlemove|mon:Squirtle,player-1,1|move:Destiny Bond"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Destiny Bond"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "singlemove".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("move".to_owned()),
                        name: "Destiny Bond".to_owned(),
                    }),
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_single_turn_volatile() {
        let state = setup_singles_battle(&["singleturn|mon:Squirtle,player-1,1|move:Protect"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Protect"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "singleturn".to_owned(),
                effect: ui::EffectData {
                    effect: Some(ui::Effect {
                        effect_type: Some("move".to_owned()),
                        name: "Protect".to_owned(),
                    }),
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 0,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_volatile_end() {
        let mut logs = Vec::from_iter(["start|mon:Squirtle,player-1,1|volatile:Substitute"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Substitute"));

        logs.push("end|mon:Squirtle,player-1,1|volatile:Substitute");
        let state = setup_singles_battle(&logs);
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.volatile_data.conditions.contains_key("Substitute"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "start".to_owned(),
                    effect: ui::EffectData {
                        effect: Some(ui::Effect {
                            effect_type: Some("volatile".to_owned()),
                            name: "Substitute".to_owned()
                        }),
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "end".to_owned(),
                    effect: ui::EffectData {
                        effect: Some(ui::Effect {
                            effect_type: Some("volatile".to_owned()),
                            name: "Substitute".to_owned(),
                        }),
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_did_not_learn_move() {
        let state = setup_singles_battle(&["didnotlearnmove|mon:Squirtle,player-1,1|move:Tackle"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.volatile_data.moves.contains(&"Tackle".to_owned()));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::MoveUpdate {
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
                move_name: "Tackle".to_owned(),
                learned: false,
                forgot: None,
            }])
        );
    }

    #[test]
    fn records_experience() {
        let state = setup_singles_battle(&["exp|mon:Squirtle,player-1,1|exp:100"]);
        let sq = squirtle_ref();
        assert_eq!(state_selectors::mon_level(&state, &sq).unwrap(), Some(5));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Experience {
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
                exp: 100,
            }])
        );
    }

    #[test]
    fn records_level_up() {
        let state =
            setup_singles_battle(&["levelup|mon:Squirtle,player-1,1|level:6|hp:20|atk:12|def:12"]);
        let sq = squirtle_ref();
        assert_eq!(state_selectors::mon_level(&state, &sq).unwrap(), Some(6));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::LevelUp {
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
                level: 6,
                stats: HashMap::from_iter([
                    ("hp".to_owned(), 20),
                    ("atk".to_owned(), 12),
                    ("def".to_owned(), 12),
                ])
            }])
        );
    }

    #[test]
    fn records_team_preview_phases() {
        let mut log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:1",
            "teampreviewstart",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        assert_eq!(state.phase, BattlePhase::PreTeamPreview);

        log.extend(["teampreview|pick:4"]).unwrap();
        let state = alter_battle_state(state, &log).unwrap();
        assert_eq!(state.phase, BattlePhase::TeamPreview(4));
        assert!(state.ui_log.iter().all(|l| l.is_empty()));
    }

    #[test]
    fn team_preview_mon_reveal_sets_flags_and_matches_on_switch() {
        let mut log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:6",
            "teamsize|player:player-2|size:6",
            "teampreviewstart",
            "mon|player:player-1|name:Bulbasaur|species:Bulbasaur|level:100|gender:F",
            "mon|player:player-1|name:Charmander|species:Charmander|level:100|gender:F",
            "mon|player:player-1|name:Squirtle|species:Squirtle|level:100|gender:F",
            "mon|player:player-1|name:Pikachu|species:Pikachu|level:100|gender:M",
            "mon|player:player-1|name:Eevee|species:Eevee|level:100|gender:M",
            "mon|player:player-1|name:Snorlax|species:Snorlax|level:100|gender:M",
            "mon|player:player-2|name:Rattata|species:Rattata|level:100|gender:M",
            "teampreview|pick:3",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();

        let p1 = &state.field.sides[0].players["player-1"];
        assert_eq!(p1.team_size, 3);
        assert_eq!(p1.mons.len(), 6);
        for mon in &p1.mons {
            assert!(mon.team_preview);
            assert!(!mon.brought);
        }
        assert_eq!(
            state_selectors::player_brought_mons(&state, "player-1")
                .unwrap()
                .count(),
            0
        );

        log.extend([
            "battlestart",
            "switch|player:player-1|position:1|name:Bulbasaur|species:Bulbasaur|level:100|gender:F|health:100/100",
            "switch|player:player-2|position:1|name:Rattata|species:Rattata|level:100|gender:M|health:100/100",
        ])
        .unwrap();
        let state = alter_battle_state(state, &log).unwrap();

        let p1 = &state.field.sides[0].players["player-1"];
        assert!(p1.mons[0].brought); // Bulbasaur
        assert!(!p1.mons[1].brought); // Charmander
        assert_eq!(
            state_selectors::player_brought_mons(&state, "player-1")
                .unwrap()
                .count(),
            1
        );

        log.extend([
            "switchout|mon:Bulbasaur,player-1,1",
            "switch|player:player-1|position:1|name:Charmander|species:Charmander|level:100|gender:F|health:100/100",
        ])
        .unwrap();
        let state = alter_battle_state(state, &log).unwrap();

        let p1 = &state.field.sides[0].players["player-1"];
        assert!(p1.mons[0].brought); // Bulbasaur
        assert!(p1.mons[1].brought); // Charmander
        assert!(!p1.mons[2].brought); // Squirtle
        assert_eq!(
            state_selectors::player_brought_mons(&state, "player-1")
                .unwrap()
                .count(),
            2
        );
    }

    #[test]
    fn team_preview_with_illusion_user() {
        let mut log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:2",
            "teamsize|player:player-2|size:1",
            "teampreviewstart",
            "mon|player:player-1|name:Bulbasaur|species:Bulbasaur|level:100|gender:F",
            "mon|player:player-1|name:Zoroark|species:Zoroark|level:100|gender:F",
            "mon|player:player-2|name:Rattata|species:Rattata|level:100|gender:M",
            "teampreview|pick:2",
            "battlestart",
            "switch|player:player-1|position:1|name:Bulbasaur|species:Bulbasaur|level:100|gender:F|health:100/100",
            "switch|player:player-2|position:1|name:Rattata|species:Rattata|level:100|gender:M|health:100/100",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();

        let p1 = &state.field.sides[0].players["player-1"];
        assert!(p1.mons[0].brought); // Bulbasaur matched
        assert!(!p1.mons[1].brought); // Zoroark not brought yet

        // Illusion breaks and reveals Zoroark via replace
        log.extend([
            "replace|player:player-1|position:1|name:Zoroark|species:Zoroark|level:100|gender:F|health:50/100",
        ])
        .unwrap();
        let state = alter_battle_state(state, &log).unwrap();

        let p1 = &state.field.sides[0].players["player-1"];
        assert!(p1.mons[0].brought); // Bulbasaur
        assert!(p1.mons[1].brought); // Zoroark matched from preview on replace
        assert_eq!(
            state_selectors::player_brought_mons(&state, "player-1")
                .unwrap()
                .count(),
            2
        );
    }

    #[test]
    fn team_preview_duplicate_species_matching() {
        let mut log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:2",
            "teamsize|player:player-2|size:1",
            "teampreviewstart",
            "mon|player:player-1|name:Pikachu|species:Pikachu|level:100|gender:M",
            "mon|player:player-1|name:Pikachu|species:Pikachu|level:100|gender:M",
            "mon|player:player-2|name:Rattata|species:Rattata|level:100|gender:M",
            "teampreview|pick:2",
            "battlestart",
            "switch|player:player-1|position:1|name:Pikachu|species:Pikachu|level:100|gender:M|health:100/100",
            "switch|player:player-2|position:1|name:Rattata|species:Rattata|level:100|gender:M|health:100/100",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();

        let p1 = &state.field.sides[0].players["player-1"];
        assert!(p1.mons[0].brought); // First Pikachu brought
        assert!(!p1.mons[1].brought); // Second Pikachu not brought yet

        log.extend([
            "switchout|mon:Pikachu,player-1,1",
            "switch|player:player-1|position:1|name:Pikachu|species:Pikachu|level:100|gender:M|health:100/100",
        ])
        .unwrap();
        let state = alter_battle_state(state, &log).unwrap();

        let p1 = &state.field.sides[0].players["player-1"];
        assert!(p1.mons[0].brought); // First Pikachu brought
        assert!(p1.mons[1].brought); // Second Pikachu now brought
    }

    #[test]
    fn team_preview_unpreviewed_mon_switch_in() {
        let mut log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:2",
            "teamsize|player:player-2|size:1",
            "teampreviewstart",
            "mon|player:player-1|name:Bulbasaur|species:Bulbasaur|level:100|gender:F",
            "mon|player:player-2|name:Rattata|species:Rattata|level:100|gender:M",
            "teampreview|pick:2",
            "battlestart",
            "switch|player:player-1|position:1|name:Bulbasaur|species:Bulbasaur|level:100|gender:F|health:100/100",
            "switch|player:player-2|position:1|name:Rattata|species:Rattata|level:100|gender:M|health:100/100",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();

        // An unexpected Mon (Mewtwo) switches in.
        log.extend([
            "switchout|mon:Bulbasaur,player-1,1",
            "switch|player:player-1|position:1|name:Mewtwo|species:Mewtwo|level:100|gender:N|health:100/100",
        ])
        .unwrap();
        let state = alter_battle_state(state, &log).unwrap();

        let p1 = &state.field.sides[0].players["player-1"];
        assert_eq!(p1.mons.len(), 2);
        assert!(p1.mons[0].team_preview);
        assert!(p1.mons[0].brought);
        assert!(!p1.mons[1].team_preview); // Mewtwo was not in Team Preview
        assert!(p1.mons[1].brought); // Mewtwo is brought in battle
    }

    #[test]
    fn records_turn_limit() {
        let state = setup_singles_battle(&["turnlimit"]);
        assert_eq!(state.phase, BattlePhase::Battle);
        assert_eq!(state.ui_log[1], Vec::from_iter([ui::UiLogEntry::TurnLimit]));
    }

    #[test]
    fn records_time_and_continue() {
        let state = setup_singles_battle(&["time", "continue"]);
        assert!(state.ui_log[1].is_empty());
    }

    #[test]
    fn records_team_member_mon_reveal() {
        let state = setup_singles_battle(&[
            "mon|player:player-1|name:Bulbasaur|species:Bulbasaur|level:5|gender:F",
        ]);
        let p1 = &state.field.sides[0].players["player-1"];
        assert_eq!(p1.mons.len(), 2);
        assert_eq!(p1.mons[1].physical_appearance.name, "Bulbasaur");
        assert!(state.ui_log[1].is_empty());
    }

    #[test]
    fn records_animate_move() {
        let state = setup_singles_battle(&["animatemove|mon:Squirtle,player-1,1|name:Tackle"]);
        let sq = squirtle_ref();
        let known_moves = state_selectors::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!known_moves.contains(&"Tackle"));
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Move {
                name: "Tackle".to_owned(),
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
                target: None,
                animate: true,
                animate_only: true,
            }])
        );
    }

    #[test]
    fn records_drag_and_appear_switches() {
        let log = Log::new(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:1",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:2",
            "teamsize|player:player-2|size:2",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "drag|player:player-1|position:1|name:Wartortle|health:100/100|species:Wartortle|level:5|gender:M",
            "appear|player:player-2|position:1|name:Charmeleon|health:100/100|species:Charmeleon|level:5|gender:M",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        let p1 = &state.field.sides[0].players["player-1"];
        let p2 = &state.field.sides[1].players["player-2"];
        assert_eq!(p1.mons[1].physical_appearance.name, "Wartortle");
        assert_eq!(p2.mons[1].physical_appearance.name, "Charmeleon");
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Switch {
                    title: "drag".to_owned(),
                    player: "player-1".to_owned(),
                    mon: 1,
                    into_position: ui::FieldPosition {
                        side: 0,
                        position: 0
                    }
                },
                ui::UiLogEntry::Switch {
                    title: "appear".to_owned(),
                    player: "player-2".to_owned(),
                    mon: 1,
                    into_position: ui::FieldPosition {
                        side: 1,
                        position: 0
                    }
                }
            ])
        );
    }

    #[test]
    fn records_switch_out_visual() {
        let state = setup_singles_battle(&["switchout|mon:Squirtle,player-1,1"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::SwitchOut {
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
            }])
        );
    }

    #[test]
    fn records_waiting_mon() {
        let state =
            setup_singles_battle(&["waiting|mon:Squirtle,player-1,1|on:Charmander,player-2,1"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Waiting {
                mon: ui::Mon::Active(ui::FieldPosition {
                    side: 0,
                    position: 0
                }),
                on: ui::Mon::Active(ui::FieldPosition {
                    side: 1,
                    position: 0
                }),
            }])
        );
    }

    #[test]
    fn records_generic_battle_effects() {
        let state = setup_singles_battle(&[
            "cant|mon:Squirtle,player-1,1|reason:Paralysis",
            "crit|mon:Squirtle,player-1,1",
            "fail|mon:Squirtle,player-1,1",
            "immune|mon:Squirtle,player-1,1",
            "miss|mon:Squirtle,player-1,1",
            "ohko",
            "protectweaken|mon:Squirtle,player-1,1",
            "resisted|mon:Squirtle,player-1,1",
            "supereffective|mon:Squirtle,player-1,1",
            "block|mon:Squirtle,player-1,1",
        ]);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "cant".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        additional: HashMap::from_iter([(
                            "reason".to_owned(),
                            "Paralysis".to_owned()
                        )]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "crit".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "fail".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "immune".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "miss".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "ohko".to_owned(),
                    effect: ui::EffectData {
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "protectweaken".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "resisted".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "supereffective".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "block".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn records_catch_failed() {
        let state = setup_singles_battle(&["catchfailed|mon:Charmander,player-2,1"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "catchfailed".to_owned(),
                effect: ui::EffectData {
                    effect: None,
                    player: None,
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_uncatchable() {
        let state = setup_singles_battle(&["uncatchable|mon:Charmander,player-2,1"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Effect {
                title: "uncatchable".to_owned(),
                effect: ui::EffectData {
                    effect: None,
                    player: None,
                    target: Some(ui::Mon::Active(ui::FieldPosition {
                        side: 1,
                        position: 0
                    })),
                    ..Default::default()
                }
            }])
        );
    }

    #[test]
    fn records_catch_rate_debug() {
        let state = setup_singles_battle(&["catchrate|rate:255"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Debug {
                title: "catchrate".to_owned(),
                values: HashMap::from_iter([("rate".to_owned(), "255".to_owned())])
            }])
        );
    }

    #[test]
    fn records_fxlang_debug() {
        let state = setup_singles_battle(&["fxlang_debug|var:val"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::Debug {
                title: "fxlang_debug".to_owned(),
                values: HashMap::from_iter([("var".to_owned(), "val".to_owned())])
            }])
        );
    }

    #[test]
    fn records_pp_adjustments() {
        let mut logs = Vec::from_iter(["deductpp|mon:Squirtle,player-1,1|move:Tackle|pp:1"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);

        logs.push("restorepp|mon:Squirtle,player-1,1|move:Tackle|pp:1");
        let state = setup_singles_battle(&logs);
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);

        logs.push("setpp|mon:Squirtle,player-1,1|move:Tackle|pp:35");
        let state = setup_singles_battle(&logs);
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.fainted);
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([
                ui::UiLogEntry::Effect {
                    title: "deductpp".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("move".to_owned()),
                            name: "Tackle".to_owned()
                        }),
                        additional: HashMap::from_iter([("pp".to_owned(), "1".to_owned())]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "restorepp".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("move".to_owned()),
                            name: "Tackle".to_owned()
                        }),
                        additional: HashMap::from_iter([("pp".to_owned(), "1".to_owned())]),
                        ..Default::default()
                    }
                },
                ui::UiLogEntry::Effect {
                    title: "setpp".to_owned(),
                    effect: ui::EffectData {
                        target: Some(ui::Mon::Active(ui::FieldPosition {
                            side: 0,
                            position: 0
                        })),
                        effect: Some(ui::Effect {
                            effect_type: Some("move".to_owned()),
                            name: "Tackle".to_owned()
                        }),
                        additional: HashMap::from_iter([("pp".to_owned(), "35".to_owned())]),
                        ..Default::default()
                    }
                }
            ])
        );
    }

    #[test]
    fn does_not_record_struggle() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Struggle|target:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        let moves = state_selectors::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!moves.contains(&"Struggle"));
    }

    #[test]
    fn does_not_record_mimic() {
        let state = setup_singles_battle(&[
            "start|mon:Squirtle,player-1,1|move:Mimic|mimic:Thunderbolt",
            "move|mon:Squirtle,player-1,1|name:Thunderbolt|target:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        let moves = state_selectors::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!moves.contains(&"Thunderbolt"));
    }

    #[test]
    fn records_moves_from_transformation() {
        let state = setup_singles_battle(&[
            "transform|mon:Squirtle,player-1,1|species:Charmander|into:Charmander,player-2,1",
            "move|mon:Squirtle,player-1,1|name:Ember|target:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        let ch = charmander_ref();

        let sq_moves = state_selectors::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!sq_moves.contains(&"Ember"));

        let ch_moves = state_selectors::mon_known_non_volatile_moves(&state, &ch)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(ch_moves.contains(&"Ember"));
    }

    #[test]
    fn records_ability_from_start() {
        let state = setup_singles_battle(&["start|mon:Squirtle,player-1,1|ability:Flash Fire"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &sq).unwrap(),
            Some("Flash Fire")
        );
    }

    #[test]
    fn records_item_from_start() {
        let state = setup_singles_battle(&["start|mon:Squirtle,player-1,1|item:Air Balloon"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_item(&state, &sq).unwrap(),
            Some("Air Balloon")
        );
    }

    #[test]
    fn fainted_mon_preserves_max_health() {
        let state = setup_singles_battle(&[
            "damage|mon:Charmander,player-2,1|health:0",
            "faint|mon:Charmander,player-2,1",
        ]);
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_health(&state, &ch).unwrap(),
            Some((0, 100))
        );
    }
}
