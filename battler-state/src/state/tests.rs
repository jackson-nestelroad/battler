#[cfg(test)]
mod state_test {
    use alloc::{
        borrow::ToOwned,
        vec::Vec,
    };

    use hashbrown::HashSet;

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
        ui_log,
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
        assert_eq!(
            state.ui_log,
            vec![
                vec![
                    ui_log!(title = "info", values = { "battletype" => "Singles" }),
                    ui_log!(title = "info", values = { "environment" => "Normal", "time" => "Evening" }),
                    ui_log!(title = "side", values = { "name" => "Side 1", "id" => 0 }),
                    ui_log!(title = "side", values = { "name" => "Side 2", "id" => 1 }),
                    ui_log!(title = "maxsidelength", values = { "length" => 1 }),
                    ui_log!(title = "player", side = 0usize, values = { "name" => "Player 1", "position" => 0, "id" => "player-1" }),
                    ui_log!(title = "player", side = 1usize, values = { "position" => 0, "name" => "Player 2", "id" => "player-2" }),
                    ui_log!(title = "teamsize", player = "player-1", values = { "size" => 3 }),
                    ui_log!(title = "teamsize", player = "player-2", values = { "size" => 3 }),
                    ui_log!(title = "battlestart"),
                ],
                vec![ui_log!(title = "turn", values = { "turn" => 1 }),],
            ]
        );
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
        assert_eq!(
            state.ui_log[1],
            vec![ui_log!(title = "turn", values = { "turn" => 1 }),]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "switch", player = "player-1", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Bulbasaur".to_owned() }, values = { "position" => 1, "gender" => "M", "name" => "Bulbasaur", "level" => 5, "mon_index" => 1, "species" => "Bulbasaur", "health" => (50, 100), "prev_mon" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }) }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "switch", player = "player-1", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Bulbasaur".to_owned() }, values = { "level" => 5, "gender" => "M", "health" => (50, 100), "position" => 1, "species" => "Bulbasaur", "name" => "Bulbasaur", "mon_index" => 1, "prev_mon" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }) }),
                ui_log!(title = "switch", player = "player-1", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Squirtle".to_owned() }, values = { "position" => 1, "gender" => "M", "health" => (100, 100), "species" => "Squirtle", "mon_index" => 0, "level" => 5, "name" => "Squirtle", "prev_mon" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Bulbasaur".to_owned() } }) }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "move", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "target" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), "name" => "Scratch" }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => (80, 100), "damage" => (20, 100) }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => 0, "damage" => (100, 100) }),
                ui_log!(
                    title = "faint",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 1usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-2".to_owned(),
                            name: "Charmander".to_owned()
                        }
                    })
                ),
            ]
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
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "switch", player = "player-2", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Bulbasaur".to_owned() }, values = { "species" => "Bulbasaur", "level" => 5, "health" => (100, 100), "name" => "Bulbasaur", "position" => 1, "gender" => "M", "mon_index" => 1, "prev_mon" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }) }),
                ui_log!(title = "residual"),
            ]
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
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "switch", player = "player-2", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Bulbasaur".to_owned() }, values = { "position" => 1, "species" => "Bulbasaur", "health" => (100, 100), "name" => "Bulbasaur", "mon_index" => 1, "gender" => "M", "level" => 5, "prev_mon" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }) }),
                ui_log!(title = "residual"),
            ]
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
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "switch", player = "player-2", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Bulbasaur".to_owned() }, values = { "position" => 1, "level" => 5, "mon_index" => 1, "name" => "Bulbasaur", "species" => "Bulbasaur", "health" => (100, 100), "gender" => "M", "prev_mon" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }) }),
                ui_log!(title = "residual"),
            ]
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
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "switch", player = "player-2", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Bulbasaur".to_owned() }, values = { "position" => 1, "species" => "Bulbasaur", "health" => (100, 100), "level" => 5, "gender" => "M", "mon_index" => 1, "name" => "Bulbasaur", "prev_mon" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }) }),
                ui_log!(title = "residual"),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => 0, "damage" => (100, 100) }),
                ui_log!(
                    title = "faint",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 1usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-2".to_owned(),
                            name: "Charmander".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => 0, "damage" => (100, 100) }),
                ui_log!(
                    title = "faint",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 1usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-2".to_owned(),
                            name: "Charmander".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "ability", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), source = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), effect = ui::Effect { effect_type: Some("ability".to_owned()), name: "Drizzle".to_owned() }, source_effect = ui::Effect { effect_type: Some("ability".to_owned()), name: "Drizzle".to_owned() }, values = { "ability" => "Drizzle" }),
            ]
        );
    }

    #[test]
    fn records_ability_from_boost() {
        let state = setup_singles_battle(&[
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2|from:ability:Moody",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &sq).unwrap(),
            Some("Moody")
        );
    }

    #[test]
    fn records_ability_from_unboost_source() {
        let state = setup_singles_battle(&[
            "unboost|mon:Squirtle,player-1,1|stat:atk|by:1|from:ability:Intimidate|of:Charmander,player-2,1",
        ]);
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &ch).unwrap(),
            Some("Intimidate")
        );
    }

    #[test]
    fn records_ability_from_move_source_effect() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1|from:ability:Dancer",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &sq).unwrap(),
            Some("Dancer")
        );
    }

    #[test]
    fn records_item_from_drag_source_effect() {
        let state = setup_singles_battle(&[
            "drag|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M|from:item:Red Card|of:Charmander,player-2,1",
        ]);
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_item(&state, &ch).unwrap(),
            Some("Red Card")
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "item", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), source = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Leftovers".to_owned() }, source_effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Leftovers".to_owned() }, values = { "item" => "Leftovers" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "ability", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), effect = ui::Effect { effect_type: Some("ability".to_owned()), name: "Blaze".to_owned() }, values = { "ability" => "Blaze" }),
            ]
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
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "move", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "target" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), "name" => "Skill Swap" }),
                ui_log!(title = "activate", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), source = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Skill Swap".to_owned() }, values = { "move" => "Skill Swap" }),
                ui_log!(title = "abilityend", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("ability".to_owned()), name: "Torrent".to_owned() }, source_effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Skill Swap".to_owned() }, values = { "ability" => "Torrent" }),
                ui_log!(title = "ability", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("ability".to_owned()), name: "Blaze".to_owned() }, source_effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Skill Swap".to_owned() }, values = { "ability" => "Blaze" }),
                ui_log!(title = "abilityend", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), source = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("ability".to_owned()), name: "Blaze".to_owned() }, source_effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Skill Swap".to_owned() }, values = { "ability" => "Blaze" }),
                ui_log!(title = "ability", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), source = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("ability".to_owned()), name: "Torrent".to_owned() }, source_effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Skill Swap".to_owned() }, values = { "ability" => "Torrent" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "activate", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("ability".to_owned()), name: "Intimidate".to_owned() }, values = { "ability" => "Intimidate" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "activate", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Quick Claw".to_owned() }, values = { "item" => "Quick Claw" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "item", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Leftovers".to_owned() }, values = { "item" => "Leftovers" }),
                ui_log!(title = "itemend", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Leftovers".to_owned() }, values = { "item" => "Leftovers" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "catch", player = "player-1", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Ultra Ball".to_owned() }, values = { "shakes" => 4, "item" => "Ultra Ball" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "stat" => "atk", "by" => 2 }),
            ]
        );
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "by" => 2, "stat" => "atk" }),
            ]
        );
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "stat" => "atk", "by" => 2 }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "weather", effect = ui::Effect { effect_type: Some("weather".to_owned()), name: "Rain".to_owned() }, values = { "weather" => "Rain" }),
            ]
        );
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "weather", effect = ui::Effect { effect_type: Some("weather".to_owned()), name: "Rain".to_owned() }, values = { "weather" => "Rain" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "status", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("status".to_owned()), name: "Paralysis".to_owned() }, values = { "status" => "Paralysis" }),
            ]
        );
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "status", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("status".to_owned()), name: "Paralysis".to_owned() }, values = { "status" => "Paralysis" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "health" => (50, 100), "damage" => (50, 100) }),
                ui_log!(title = "heal", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "health" => (75, 100), "heal" => (25, 100) }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "start", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("volatile".to_owned()), name: "Substitute".to_owned() }, values = { "volatile" => "Substitute" }),
            ]
        );
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "start", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("volatile".to_owned()), name: "Substitute".to_owned() }, values = { "volatile" => "Substitute" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "fieldstart", effect = ui::Effect { effect_type: Some("condition".to_owned()), name: "Trick Room".to_owned() }, values = { "condition" => "Trick Room" }),
            ]
        );
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "fieldstart", effect = ui::Effect { effect_type: Some("condition".to_owned()), name: "Trick Room".to_owned() }, values = { "condition" => "Trick Room" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "formechange", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Squirtle-Mega".to_owned() }, values = { "species" => "Squirtle-Mega" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "item", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Leftovers".to_owned() }, values = { "item" => "Leftovers" }),
                ui_log!(title = "itemend", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Leftovers".to_owned() }, values = { "item" => "Leftovers" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "prepare", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Solar Beam".to_owned() }, values = { "move" => "Solar Beam" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "singlemove", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Destiny Bond".to_owned() }, values = { "move" => "Destiny Bond" }),
            ]
        );
    }

    #[test]
    fn removes_single_move_volatile_on_next_move() {
        let mut logs = Vec::from_iter([
            "move|mon:Squirtle,player-1,1|name:Destiny Bond|target:Charmander,player-2,1",
            "singlemove|mon:Squirtle,player-1,1|move:Destiny Bond",
        ]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Destiny Bond"));

        logs.push("move|mon:Squirtle,player-1,1|name:Tackle|target:Charmander,player-2,1");
        let state = setup_singles_battle(&logs);
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.volatile_data.conditions.contains_key("Destiny Bond"));
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
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "move", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "name" => "Metronome", "target" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }) }),
                ui_log!(title = "move", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), source_effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Metronome".to_owned() }, values = { "target" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), "name" => "Ice Beam" }),
            ]
        );
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "move", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "target" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), "name" => "Metronome" }),
                ui_log!(title = "move", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), source_effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Metronome".to_owned() }, values = { "target" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), "name" => "Ice Beam" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "transform", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Charmander".to_owned() }, values = { "into" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), "species" => "Charmander" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "typechange", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "types" => "Fire/Flying" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "cannotescape", player = "player-1"),
            ]
        );

        log.extend(["escaped|player:player-1"]).unwrap();
        let state = alter_battle_state(state, &log).unwrap();
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "cannotescape", player = "player-1"),
            ]
        );
    }

    #[test]
    fn records_forfeit() {
        let state = setup_singles_battle(&["forfeited|player:player-1"]);
        assert_eq!(state.phase, BattlePhase::Battle);
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "forfeited", player = "player-1", values = { "positions" => "0" }),
            ]
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
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "move", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "name" => "Pound" }),
            ]
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
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "move", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "target" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), "name" => "Double Slap" }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => (90, 100), "damage" => (10, 100) }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => (80, 100), "damage" => (10, 100) }),
                ui_log!(title = "hitcount", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "count" => 2 }),
            ]
        );
    }

    #[test]
    fn records_tie() {
        let state = setup_singles_battle(&["tie"]);
        assert_eq!(state.phase, BattlePhase::Finished);
        assert_eq!(state.winning_side, None);
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "tie"),
            ]
        );
    }

    #[test]
    fn records_win() {
        let state = setup_singles_battle(&["win|side:0"]);
        assert_eq!(state.phase, BattlePhase::Finished);
        assert_eq!(state.winning_side, Some(0));
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "win", side = 0usize),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "useitem", player = "player-1", values = { "target" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), "name" => "Oran Berry" }),
            ]
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

        logs.push("copyboosts|mon:Squirtle,player-1,1|source:Charmander,player-2,1");
        let state = setup_singles_battle(&logs);
        let boosts = state_selectors::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "by" => 2, "stat" => "atk" }),
                ui_log!(
                    title = "copyboosts",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    }),
                    values = {
                        "source" => ui::Mon::Active(ui::ActiveMonReference {
                            position: ui::FieldPosition {
                                side: 1usize,
                                position: 0usize
                            },
                            reference: ui::MonReference {
                                player: "player-2".to_owned(),
                                name: "Charmander".to_owned()
                            }
                        })
                    }
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "by" => 2, "stat" => "atk" }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "by" => 1, "stat" => "def" }),
                ui_log!(
                    title = "swapboosts",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    }),
                    source = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 1usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-2".to_owned(),
                            name: "Charmander".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "by" => 2, "stat" => "atk" }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "stat" => "def", "by" => 1 }),
                ui_log!(title = "swapboosts", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), source = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "stats" => "atk" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "mega", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("item".to_owned()), name: "Squirtlite".to_owned() }, values = { "species" => "Squirtle-Mega", "item" => "Squirtlite" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(
                    title = "dynamax",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "gigantamax", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Squirtle-Gmax".to_owned() }, values = { "species" => "Squirtle-Gmax" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "tera", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("type".to_owned()), name: "Fire".to_owned() }, values = { "type" => "Fire" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "timer", values = { "source" => "-battlerservice", "battle" => true, "remainingsecs" => 5 }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "by" => 2, "stat" => "atk" }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "stat" => "def", "by" => -1 }),
                ui_log!(
                    title = "invertboosts",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(
                    title = "clearpositiveboosts",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(title = "addedtype", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("type".to_owned()), name: "Grass".to_owned() }, values = { "type" => "Grass" }),
                ui_log!(title = "swapplayer", player = "player-1", values = { "position" => 2 }),
            ]
        );
    }

    #[test]
    fn records_field_activate() {
        let state = setup_singles_battle(&["fieldactivate|effect:Gravity"]);
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "fieldactivate", values = { "effect" => "Gravity" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "by" => 2, "stat" => "atk" }),
                ui_log!(
                    title = "clearboosts",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "stat" => "atk", "by" => 2 }),
                ui_log!(title = "boost", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "by" => -2, "stat" => "def" }),
                ui_log!(
                    title = "clearnegativeboosts",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "weather", effect = ui::Effect { effect_type: Some("weather".to_owned()), name: "RainDance".to_owned() }, values = { "weather" => "RainDance" }),
                ui_log!(title = "clearweather"),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "health" => 0, "damage" => (100, 100) }),
                ui_log!(
                    title = "faint",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(title = "revive", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "health" => (50, 100) }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "sethp", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "health" => (42, 100) }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "primal", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Squirtle-Primal".to_owned() }, values = { "species" => "Squirtle-Primal" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "ultra", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Squirtle-Ultra".to_owned() }, values = { "species" => "Squirtle-Ultra" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(
                    title = "dynamax",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(
                    title = "revertdynamax",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "revertgigantamax", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Squirtle".to_owned() }, values = { "species" => "Squirtle" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "revertmega", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Squirtle".to_owned() }, values = { "species" => "Squirtle" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "tera", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("type".to_owned()), name: "Fire".to_owned() }, values = { "type" => "Fire" }),
                ui_log!(
                    title = "reverttera",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "specieschange", player = "player-1", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Wartortle".to_owned() }, values = { "gender" => "M", "species" => "Wartortle", "name" => "Squirtle", "position" => 1, "level" => 5, "health" => (100, 100) }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "typechange", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "types" => "Fire/Flying" }),
                ui_log!(
                    title = "resettypechange",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
            ]
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
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "swap", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "position" => 2 }),
            ]
        );
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
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "sidestart", side = 0usize, effect = ui::Effect { effect_type: Some("condition".to_owned()), name: "Spikes".to_owned() }, values = { "condition" => "Spikes" }),
                ui_log!(title = "swapsideconditions", side = 0usize, values = { "with" => 1 }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "singleturn", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Protect".to_owned() }, values = { "move" => "Protect" }),
            ]
        );
    }

    #[test]
    fn removes_single_turn_volatile_on_next_turn() {
        let mut logs = Vec::from_iter(["singleturn|mon:Squirtle,player-1,1|move:Protect"]);
        let state = setup_singles_battle(&logs);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Protect"));

        logs.push("turn|turn:2");
        let state = setup_singles_battle(&logs);
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(!sq_mon.volatile_data.conditions.contains_key("Protect"));
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "start", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("volatile".to_owned()), name: "Substitute".to_owned() }, values = { "volatile" => "Substitute" }),
                ui_log!(title = "end", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("volatile".to_owned()), name: "Substitute".to_owned() }, values = { "volatile" => "Substitute" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "didnotlearnmove", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Tackle".to_owned() }, values = { "move" => "Tackle" }),
            ]
        );
    }

    #[test]
    fn records_experience() {
        let state = setup_singles_battle(&["exp|mon:Squirtle,player-1,1|exp:100"]);
        let sq = squirtle_ref();
        assert_eq!(state_selectors::mon_level(&state, &sq).unwrap(), Some(5));
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "exp", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "exp" => 100 }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "levelup", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "def" => 12, "atk" => 12, "level" => 6, "hp" => 20 }),
            ]
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
        assert_eq!(
            state.ui_log[0],
            vec![
                ui_log!(title = "info", values = { "battletype" => "Singles" }),
                ui_log!(title = "side", values = { "id" => 0, "name" => "Side 1" }),
                ui_log!(title = "side", values = { "id" => 1, "name" => "Side 2" }),
                ui_log!(title = "maxsidelength", values = { "length" => 1 }),
                ui_log!(title = "player", side = 0usize, values = { "position" => 0, "name" => "Player 1", "id" => "player-1" }),
                ui_log!(title = "player", side = 1usize, values = { "name" => "Player 2", "id" => "player-2", "position" => 0 }),
                ui_log!(title = "teamsize", player = "player-1", values = { "size" => 1 }),
                ui_log!(title = "teamsize", player = "player-2", values = { "size" => 1 }),
                ui_log!(title = "teampreviewstart", values = {}),
                ui_log!(title = "teampreview", values = { "pick" => 4 })
            ]
        );
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
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "turnlimit"),
            ]
        );
    }

    #[test]
    fn records_time_and_continue() {
        let state = setup_singles_battle(&["time", "continue"]);
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "time"),
                ui_log!(title = "continue"),
            ]
        );
    }

    #[test]
    fn records_team_member_mon_reveal() {
        let state = setup_singles_battle(&[
            "mon|player:player-1|name:Bulbasaur|species:Bulbasaur|level:5|gender:F",
        ]);
        let p1 = &state.field.sides[0].players["player-1"];
        assert_eq!(p1.mons.len(), 2);
        assert_eq!(p1.mons[1].physical_appearance.name, "Bulbasaur");
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "mon", player = "player-1", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Bulbasaur".to_owned() }, values = { "name" => "Bulbasaur", "level" => 5, "gender" => "F", "species" => "Bulbasaur" }),
            ]
        );
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "animatemove", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "name" => "Tackle" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "drag", player = "player-1", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Wartortle".to_owned() }, values = { "species" => "Wartortle", "gender" => "M", "position" => 1, "mon_index" => 1, "health" => (100, 100), "level" => 5, "name" => "Wartortle" }),
                ui_log!(title = "appear", player = "player-2", effect = ui::Effect { effect_type: Some("species".to_owned()), name: "Charmeleon".to_owned() }, values = { "level" => 5, "name" => "Charmeleon", "gender" => "M", "position" => 1, "health" => (100, 100), "mon_index" => 1, "species" => "Charmeleon" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(
                    title = "switchout",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "waiting", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "on" => ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }) }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "cant", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), values = { "reason" => "Paralysis" }),
                ui_log!(
                    title = "crit",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(
                    title = "fail",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(
                    title = "immune",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(
                    title = "miss",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(title = "ohko"),
                ui_log!(
                    title = "protectweaken",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(
                    title = "resisted",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(
                    title = "supereffective",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
                ui_log!(
                    title = "block",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(
                    title = "catchfailed",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 1usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-2".to_owned(),
                            name: "Charmander".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(
                    title = "uncatchable",
                    target = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 1usize,
                            position: 0usize
                        },
                        reference: ui::MonReference {
                            player: "player-2".to_owned(),
                            name: "Charmander".to_owned()
                        }
                    })
                ),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "catchrate", values = { "rate" => 255 }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "fxlang_debug", values = { "var" => "val" }),
            ]
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
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "deductpp", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Tackle".to_owned() }, values = { "move" => "Tackle", "pp" => 1 }),
                ui_log!(title = "restorepp", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Tackle".to_owned() }, values = { "move" => "Tackle", "pp" => 1 }),
                ui_log!(title = "setpp", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 0usize, position: 0usize }, reference: ui::MonReference { player: "player-1".to_owned(), name: "Squirtle".to_owned() } }), effect = ui::Effect { effect_type: Some("move".to_owned()), name: "Tackle".to_owned() }, values = { "pp" => 35, "move" => "Tackle" }),
            ]
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
    fn records_ability_and_source_ability_from_abilitystart() {
        let state = setup_singles_battle(&[
            "abilitystart|mon:Squirtle,player-1,1|ability:Sticky Hold|source:Charmander,player-2,1|from:move:Role Play",
        ]);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &sq).unwrap(),
            Some("Sticky Hold")
        );
        assert_eq!(
            state_selectors::mon_ability(&state, &ch).unwrap(),
            Some("Sticky Hold")
        );

        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert_eq!(sq_mon.volatile_data.ability.as_deref(), Some("Sticky Hold"));

        let ch_mon = state.field.mon_by_reference_or_else(&ch).unwrap();
        assert_eq!(ch_mon.volatile_data.ability, None);
        assert_eq!(
            state_selectors::mon_battle_appearance_or_else(&state, &ch)
                .unwrap()
                .ability
                .known(),
            Some(&"Sticky Hold".to_owned())
        );
    }

    #[test]
    fn records_ability_from_abilitystart_without_source() {
        let state = setup_singles_battle(&[
            "abilitystart|mon:Squirtle,player-1,1|ability:Insomnia|from:move:Worry Seed|of:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        assert_eq!(
            state_selectors::mon_ability(&state, &sq).unwrap(),
            Some("Insomnia")
        );
        assert_eq!(state_selectors::mon_ability(&state, &ch).unwrap(), None);
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

    #[test]
    fn records_damage_amount() {
        let state = setup_singles_battle(&[
            "damage|mon:Charmander,player-2,1|health:73/100",
            "damage|mon:Charmander,player-2,1|health:0",
        ]);
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => (73, 100), "damage" => (27, 100) }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => 0, "damage" => (73, 100) }),
            ]
        );
    }

    #[test]
    fn records_heal_amount() {
        let state = setup_singles_battle(&[
            "damage|mon:Charmander,player-2,1|health:50/100",
            "heal|mon:Charmander,player-2,1|health:85/100",
        ]);
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(title = "damage", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => (50, 100), "damage" => (50, 100) }),
                ui_log!(title = "heal", target = ui::Mon::Active(ui::ActiveMonReference { position: ui::FieldPosition { side: 1usize, position: 0usize }, reference: ui::MonReference { player: "player-2".to_owned(), name: "Charmander".to_owned() } }), values = { "health" => (85, 100), "heal" => (35, 100) }),
            ]
        );
    }

    #[test]
    fn records_previous_mon_on_switch() {
        let state = setup_singles_battle(&[
            "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
        ]);
        let switch_log = &state.ui_log[1][1];
        assert_eq!(switch_log.title, "switch");
        assert_eq!(
            switch_log.values.get("prev_mon"),
            Some(&ui::LogValue::Mon(ui::Mon::Active(
                ui::ActiveMonReference {
                    position: ui::FieldPosition {
                        side: 0,
                        position: 0,
                    },
                    reference: ui::MonReference {
                        player: "player-1".to_owned(),
                        name: "Squirtle".to_owned(),
                    },
                }
            )))
        );
    }

    #[test]
    fn does_not_record_previous_mon_on_initial_switch() {
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
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        let p1_switch = &state.ui_log[0][10];
        assert_eq!(p1_switch.title, "switch");
        assert_eq!(p1_switch.values.get("prev_mon"), None);
    }

    #[test]
    fn does_not_record_previous_mon_after_switchout() {
        let state = setup_singles_battle(&[
            "switchout|mon:Squirtle,player-1,1",
            "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
        ]);
        let switch_log = &state.ui_log[1][2];
        assert_eq!(switch_log.title, "switch");
        assert_eq!(switch_log.values.get("prev_mon"), None);
    }

    #[test]
    fn does_not_record_previous_mon_after_faint() {
        let state = setup_singles_battle(&[
            "damage|mon:Squirtle,player-1,1|health:0",
            "faint|mon:Squirtle,player-1,1",
            "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
        ]);
        let switch_log = &state.ui_log[1][3];
        assert_eq!(switch_log.title, "switch");
        assert_eq!(switch_log.values.get("prev_mon"), None);
    }

    #[test]
    fn records_wild_player_type() {
        let log = Log::new([
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:wild-1|name:Wild|side:1|position:0|wild",
        ])
        .unwrap();
        let state = alter_battle_state(BattleState::default(), &log).unwrap();
        assert_eq!(
            state.field.sides[0].players.get("player-1").unwrap().wild,
            false
        );
        assert_eq!(
            state.field.sides[1].players.get("wild-1").unwrap().wild,
            true
        );
    }

    #[test]
    fn records_condition_start_and_end_without_mon() {
        let state = setup_singles_battle(&[
            "start|move:Future Sight|of:Squirtle,player-1,1",
            "end|move:Future Sight|of:Squirtle,player-1,1",
        ]);
        assert_eq!(
            state.ui_log[1],
            vec![
                ui_log!(title = "turn", values = { "turn" => 1 }),
                ui_log!(
                    title = "start",
                    source = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize,
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned(),
                        },
                    }),
                    effect = ui::Effect {
                        effect_type: Some("move".to_owned()),
                        name: "Future Sight".to_owned(),
                    },
                    values = { "move" => "Future Sight" }
                ),
                ui_log!(
                    title = "end",
                    source = ui::Mon::Active(ui::ActiveMonReference {
                        position: ui::FieldPosition {
                            side: 0usize,
                            position: 0usize,
                        },
                        reference: ui::MonReference {
                            player: "player-1".to_owned(),
                            name: "Squirtle".to_owned(),
                        },
                    }),
                    effect = ui::Effect {
                        effect_type: Some("move".to_owned()),
                        name: "Future Sight".to_owned(),
                    },
                    values = { "move" => "Future Sight" }
                ),
            ]
        );
    }
}
