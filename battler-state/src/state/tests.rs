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
        state_util,
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
        let mut logs = vec![
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
        ];
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
    }

    #[test]
    fn records_simple_move_and_damage() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:75/100",
        ]);
        let ch = charmander_ref();
        assert_eq!(
            state_util::mon_health(&state, &ch).unwrap(),
            Some((75, 100))
        );
        let sq = squirtle_ref();
        let moves = state_util::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(moves.contains(&"Pound"));
    }

    #[test]
    fn records_new_mon_revealed_from_switch() {
        let state = setup_singles_battle(&[
            "switch|player:player-1|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:5|gender:M",
        ]);
        let p1_mons = &state.field.sides[0].players.get("player-1").unwrap().mons;
        assert_eq!(p1_mons.len(), 2);
        assert_eq!(p1_mons[1].physical_appearance.name, "Bulbasaur");
    }

    #[test]
    fn uses_old_mon_reappeared_from_switch() {
        let state = setup_singles_battle(&[
            "switch|player:player-1|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:5|gender:M",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
        ]);
        let p1_mons = &state.field.sides[0].players.get("player-1").unwrap().mons;
        assert_eq!(p1_mons.len(), 2);
        assert_eq!(
            state.field.sides[0].active[0].as_ref().unwrap().mon_index,
            0
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
        let p2 = &state.field.sides[1].players["player-2"];
        assert_eq!(
            p2.mons[0].battle_appearances[0]
                .primary()
                .moves
                .known()
                .iter()
                .collect::<Vec<_>>(),
            vec!["Growl"]
        );
        assert_eq!(
            p2.mons[1].battle_appearances[0]
                .primary()
                .moves
                .known()
                .iter()
                .collect::<Vec<_>>(),
            vec!["Absorb"]
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
        let p2 = &state.field.sides[1].players["player-2"];
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
    }

    #[test]
    fn illusion_user_faints_before_being_revealed() {
        let state = setup_singles_battle(&[
            "damage|mon:Charmander,player-2,1|health:0",
            "faint|mon:Charmander,player-2,1",
        ]);
        let p2 = &state.field.sides[1].players["player-2"];
        assert!(p2.mons[0].fainted);
    }

    #[test]
    fn corrects_fainted_illusion_user_with_multiple_illusion_users() {
        let state = setup_singles_battle(&[
            "damage|mon:Charmander,player-2,1|health:0",
            "faint|mon:Charmander,player-2,1",
        ]);
        assert!(state.field.sides[1].players["player-2"].mons[0].fainted);
    }

    #[test]
    fn records_ability_from_source_effect() {
        let state = setup_singles_battle(&[
            "ability|mon:Squirtle,player-1,1|ability:Drizzle|from:ability:Drizzle|of:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_util::mon_ability(&state, &sq).unwrap(),
            Some("Drizzle")
        );
    }

    #[test]
    fn records_item_from_source_effect() {
        let state = setup_singles_battle(&[
            "item|mon:Squirtle,player-1,1|item:Leftovers|from:item:Leftovers|of:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_util::mon_item(&state, &sq).unwrap(),
            Some("Leftovers")
        );
    }

    #[test]
    fn records_ability() {
        let state = setup_singles_battle(&["ability|mon:Charmander,player-2,1|ability:Blaze"]);
        let ch = charmander_ref();
        assert_eq!(state_util::mon_ability(&state, &ch).unwrap(), Some("Blaze"));
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
    }

    #[test]
    fn records_ability_from_activation() {
        let state = setup_singles_battle(&["activate|mon:Squirtle,player-1,1|ability:Intimidate"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_util::mon_ability(&state, &sq).unwrap(),
            Some("Intimidate")
        );
    }

    #[test]
    fn records_item_from_activation() {
        let state = setup_singles_battle(&["activate|mon:Squirtle,player-1,1|item:Quick Claw"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_util::mon_item(&state, &sq).unwrap(),
            Some("Quick Claw")
        );
    }

    #[test]
    fn does_not_record_item_after_item_end_log() {
        let state = setup_singles_battle(&[
            "-item|mon:Squirtle,player-1,1|item:Leftovers",
            "-itemend|mon:Squirtle,player-1,1|item:Leftovers",
        ]);
        let sq = squirtle_ref();
        assert_eq!(state_util::mon_item(&state, &sq).unwrap(), None);
    }

    #[test]
    fn records_and_switches_out_caught_mon() {
        let state = setup_singles_battle(&[
            "catch|player:player-1|mon:Charmander,player-2,1|item:Ultra Ball|shakes:4",
        ]);
        assert!(state.field.sides[1].active[0].is_some());
        assert!(state.field.sides[1].players["player-2"].mons[0].fainted);
    }

    #[test]
    fn records_stat_boosts() {
        let state = setup_singles_battle(&[
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
            "turn|turn:2",
            "boost|mon:Squirtle,player-1,1|stat:def|by:-1",
            "turn|turn:3",
            "clearallboosts",
            "turn|turn:4",
        ]);
        let sq = squirtle_ref();
        let boosts = state_util::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 0);
        assert_eq!(boosts.get(battler::Boost::Def), 0);
    }

    #[test]
    fn records_weather() {
        let state = setup_singles_battle(&[
            "weather|weather:Rain",
            "turn|turn:2",
            "clearweather",
            "turn|turn:3",
        ]);
        assert_eq!(state_util::field_weather(&state), None);
    }

    #[test]
    fn records_status() {
        let state = setup_singles_battle(&[
            "status|mon:Squirtle,player-1,1|status:Paralysis",
            "turn|turn:2",
            "curestatus|mon:Squirtle,player-1,1|status:Paralysis",
            "turn|turn:3",
        ]);
        let sq = squirtle_ref();
        assert_eq!(state_util::mon_status(&state, &sq).unwrap(), Some(""));
    }

    #[test]
    fn records_health_changes() {
        let state = setup_singles_battle(&[
            "damage|mon:Squirtle,player-1,1|health:50/100",
            "heal|mon:Squirtle,player-1,1|health:75/100",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_util::mon_health(&state, &sq).unwrap(),
            Some((75, 100))
        );
    }

    #[test]
    fn records_volatile_condition() {
        let state = setup_singles_battle(&[
            "start|mon:Squirtle,player-1,1|volatile:Substitute",
            "turn|turn:2",
            "end|mon:Squirtle,player-1,1|volatile:Substitute",
            "turn|turn:3",
        ]);
        let sq = squirtle_ref();
        let conds = state_util::mon_conditions(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!conds.contains(&"Substitute"));
    }

    #[test]
    fn records_field_condition() {
        let state = setup_singles_battle(&[
            "fieldstart|condition:Trick Room",
            "turn|turn:2",
            "fieldend|condition:Trick Room",
            "turn|turn:3",
        ]);
        let conds = state_util::field_conditions(&state).collect::<Vec<_>>();
        assert!(!conds.contains(&"Trick Room"));
    }

    #[test]
    fn records_forme_change() {
        let state =
            setup_singles_battle(&["formechange|mon:Squirtle,player-1,1|species:Squirtle-Mega"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_util::mon_species(&state, &sq).unwrap(),
            "Squirtle-Mega"
        );
    }

    #[test]
    fn records_item_changes() {
        let state = setup_singles_battle(&[
            "item|mon:Squirtle,player-1,1|item:Leftovers",
            "itemend|mon:Squirtle,player-1,1|item:Leftovers",
        ]);
        let sq = squirtle_ref();
        assert_eq!(state_util::mon_item(&state, &sq).unwrap(), Some(""));
    }

    #[test]
    fn records_move_volatile_with_prepare() {
        let state = setup_singles_battle(&["prepare|mon:Squirtle,player-1,1|move:Solar Beam"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Solar Beam"));
    }

    #[test]
    fn records_move_volatile_until_next_move() {
        let state = setup_singles_battle(&["singlemove|mon:Squirtle,player-1,1|move:Destiny Bond"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Destiny Bond"));
    }

    #[test]
    fn does_not_record_externally_used_move() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Metronome|target:Charmander,player-2,1",
            "move|mon:Squirtle,player-1,1|name:Ice Beam|target:Charmander,player-2,1|from:move:Metronome",
        ]);
        let sq = squirtle_ref();
        let moves = state_util::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!moves.contains(&"Ice Beam"));
    }

    #[test]
    fn records_side_condition() {
        let state = setup_singles_battle(&[
            "sidestart|side:0|condition:Spikes",
            "turn|turn:2",
            "sideend|side:0|condition:Spikes",
            "turn|turn:3",
        ]);
        let conds = state_util::side_conditions(&state, 0)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!conds.contains(&"Spikes"));
    }

    #[test]
    fn records_transformation() {
        let state = setup_singles_battle(&[
            "transform|mon:Squirtle,player-1,1|species:Charmander|into:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.transformed.is_some());
    }

    #[test]
    fn records_type_change() {
        let state = setup_singles_battle(&["typechange|mon:Squirtle,player-1,1|types:Fire/Flying"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert_eq!(sq_mon.volatile_data.types, vec!["Fire", "Flying"]);
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
        assert_eq!(
            state.ui_log[1],
            Vec::from_iter([ui::UiLogEntry::CannotEscape {
                player: "player-1".to_owned()
            }])
        );

        log.extend(["escaped|player:player-1"]).unwrap();
        let state = alter_battle_state(state, &log).unwrap();
        if let Some(ui::UiLogEntry::Leave { player, .. }) = state.ui_log[2].first() {
            assert_eq!(player, "player-1");
        } else {
            panic!("expected Leave");
        }
    }

    #[test]
    fn records_forfeit() {
        let state = setup_singles_battle(&["forfeited|player:player-1"]);
        if let Some(ui::UiLogEntry::Leave { player, .. }) = state.ui_log[1].first() {
            assert_eq!(player, "player-1");
        } else {
            panic!("expected Leave");
        }
    }

    #[test]
    fn records_learned_move() {
        let state = setup_singles_battle(&[
            "move|mon:Squirtle,player-1,1|name:Pound",
            "turn|turn:2",
            "learnedmove|mon:Squirtle,player-1,1|move:Water Gun|forgot:Pound",
        ]);
        let sq = squirtle_ref();
        let moves = state_util::mon_known_non_volatile_moves(&state, &sq)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(moves.contains(&"Water Gun"));
        assert!(!moves.contains(&"Pound"));
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
            state_util::mon_health(&state, &ch).unwrap(),
            Some((80, 100))
        );
    }

    #[test]
    fn records_tie() {
        let state = setup_singles_battle(&["tie"]);
        assert_eq!(state.phase, BattlePhase::Finished);
        assert_eq!(state.winning_side, None);
    }

    #[test]
    fn records_win() {
        let state = setup_singles_battle(&["win|side:0"]);
        assert_eq!(state.phase, BattlePhase::Finished);
        assert_eq!(state.winning_side, Some(0));
    }

    #[test]
    fn records_use_item() {
        let state = setup_singles_battle(&[
            "useitem|player:player-1|name:Oran Berry|target:Squirtle,player-1,1",
        ]);
        if let Some(ui::UiLogEntry::UseItem { player, item, .. }) = state.ui_log[1].first() {
            assert_eq!(player, "player-1");
            assert_eq!(item, "Oran Berry");
        } else {
            panic!("expected UseItem");
        }
    }

    #[test]
    fn records_copied_boosts() {
        let state = setup_singles_battle(&[
            "boost|mon:Charmander,player-2,1|stat:atk|by:2",
            "copyboosts|mon:Squirtle,player-1,1|of:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        let boosts = state_util::mon_boosts(&state, &sq).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), 2);
    }

    #[test]
    fn records_swapped_boosts_for_all_stats() {
        let state = setup_singles_battle(&[
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
            "boost|mon:Charmander,player-2,1|stat:def|by:1",
            "swapboosts|mon:Squirtle,player-1,1|of:Charmander,player-2,1",
        ]);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        assert_eq!(
            state_util::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Atk),
            0
        );
        assert_eq!(
            state_util::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Def),
            1
        );
        assert_eq!(
            state_util::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Atk),
            2
        );
    }

    #[test]
    fn records_swapped_boosts_for_some_stats() {
        let state = setup_singles_battle(&[
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
            "boost|mon:Charmander,player-2,1|stat:def|by:1",
            "swapboosts|mon:Squirtle,player-1,1|of:Charmander,player-2,1|stats:atk",
        ]);
        let sq = squirtle_ref();
        let ch = charmander_ref();
        assert_eq!(
            state_util::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Atk),
            0
        );
        assert_eq!(
            state_util::mon_boosts(&state, &sq)
                .unwrap()
                .get(battler::Boost::Def),
            0
        );
        assert_eq!(
            state_util::mon_boosts(&state, &ch)
                .unwrap()
                .get(battler::Boost::Atk),
            2
        );
    }

    #[test]
    fn records_mega_evolution() {
        let state = setup_singles_battle(&[
            "mega|mon:Squirtle,player-1,1|species:Squirtle-Mega|item:Squirtlite",
        ]);
        let sq = squirtle_ref();
        assert_eq!(
            state_util::mon_species(&state, &sq).unwrap(),
            "Squirtle-Mega"
        );
    }

    #[test]
    fn records_dynamax() {
        let state = setup_singles_battle(&["dynamax|mon:Squirtle,player-1,1"]);
        let sq = squirtle_ref();
        let sq_mon = state.field.mon_by_reference_or_else(&sq).unwrap();
        assert!(sq_mon.volatile_data.conditions.contains_key("Dynamax"));
    }

    #[test]
    fn records_gigantamax() {
        let state =
            setup_singles_battle(&["gigantamax|mon:Squirtle,player-1,1|species:Squirtle-Gmax"]);
        let sq = squirtle_ref();
        assert_eq!(
            state_util::mon_species(&state, &sq).unwrap(),
            "Squirtle-Gmax"
        );
    }

    #[test]
    fn records_terastallization() {
        let state = setup_singles_battle(&["tera|mon:Squirtle,player-1,1|type:Fire"]);
        let sq = squirtle_ref();
        let sq_app = state_util::mon_battle_appearance_or_else(&state, &sq).unwrap();
        assert_eq!(
            sq_app.terastallization.known().map(|s| s.as_str()),
            Some("Fire")
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
        let state = setup_singles_battle(&[
            "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
            "boost|mon:Squirtle,player-1,1|stat:def|by:-1",
            "invertboosts|mon:Squirtle,player-1,1",
            "clearpositiveboosts|mon:Squirtle,player-1,1",
            "addvolatile|mon:Squirtle,player-1,1|volatile:Substitute",
            "addedtype|mon:Squirtle,player-1,1|type:Grass",
            "addsidecondition|side:0|condition:Spikes",
            "addslotcondition|side:0|slot:0|condition:Wish",
            "swapplayer|player:player-1|position:2",
            "turn|turn:2",
        ]);

        let squirtle_ref = squirtle_ref();
        let boosts = state_util::mon_boosts(&state, &squirtle_ref).unwrap();
        assert_eq!(boosts.get(battler::Boost::Atk), -2);
        assert_eq!(boosts.get(battler::Boost::Def), 0);

        let conditions = state_util::mon_conditions(&state, &squirtle_ref)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(conditions.contains(&"Substitute"));

        let data = battler_test_utils::static_local_data_store();
        let types = state_util::mon_types(&state, &squirtle_ref, data).unwrap();
        assert!(types.contains(&battler::Type::Water));
        assert!(types.contains(&battler::Type::Grass));

        let side_conds = state_util::side_conditions(&state, 0)
            .unwrap()
            .collect::<Vec<_>>();
        assert!(side_conds.contains(&"Spikes"));

        assert!(state.field.sides[0].slot_conditions[0].contains_key("Wish"));

        assert_eq!(
            state.field.sides[0]
                .players
                .get("player-1")
                .unwrap()
                .position,
            2
        );
    }
}
