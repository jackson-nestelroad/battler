#[cfg(test)]
mod team_preview_tests {
    use battler::{
        battle::{
            BattleType,
            PublicCoreBattle,
        },
        common::{
            Error,
            FastHashMap,
            WrapResultError,
        },
        dex::{
            DataStore,
            LocalDataStore,
        },
        teams::TeamData,
    };
    use battler_test_utils::{
        assert_error_message,
        BattleIoVerifier,
        TestBattleBuilder,
    };

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur F",
                        "species": "Bulbasaur",
                        "ability": "Chlorophyll",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 100
                    },
                    {
                        "name": "Charmander F",
                        "species": "Charmander",
                        "ability": "Blaze",
                        "moves": ["Scratch"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 100
                    },
                    {
                        "name": "Squirtle F",
                        "species": "Squirtle",
                        "ability": "Torrent",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 100
                    },
                    {
                        "name": "Bulbasaur M",
                        "species": "Bulbasaur",
                        "ability": "Chlorophyll",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    },
                    {
                        "name": "Charmander M",
                        "species": "Charmander",
                        "ability": "Blaze",
                        "moves": ["Scratch"],
                        "nature": "Modest",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    },
                    {
                        "name": "Squirtle M",
                        "species": "Squirtle",
                        "ability": "Torrent",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn make_multi_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Multi)
            .with_rule("Standard")
            .with_rule("! Species Clause")
            .with_rule("Force Level = 100")
            .with_rule("Min Team Size = 3")
            .with_rule("Picked Team Size = 3")
            .with_rule("Team Preview")
            .with_seed(0)
            .with_auto_continue(false)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_1("player-2", "Player 2")
            .add_player_to_side_2("player-3", "Player 3")
            .add_player_to_side_2("player-4", "Player 4")
            // All players have the same team. We are testing that each player can pick a different
            // order.
            .with_team("player-1", team()?)
            .with_team("player-2", team()?)
            .with_team("player-3", team()?)
            .with_team("player-4", team()?)
            .build(data)
    }

    #[test]
    fn team_preview_orders_all_player_teams() {
        let mut battle_io = BattleIoVerifier::new("team_preview_test.json").unwrap();
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_multi_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));
        battle_io.verify_next_request_set(&mut battle);

        assert_eq!(battle.ready_to_continue(), Ok(false));
        battle_io.verify_new_logs(&mut battle);

        // Player 1 made their choice.
        assert_eq!(battle.set_player_choice("player-1", "team 0 1 2"), Ok(()));
        assert!(!battle
            .active_requests()
            .collect::<FastHashMap<_, _>>()
            .contains_key("player-1"));
        assert_eq!(battle.ready_to_continue(), Ok(false));
        assert!(!battle.has_new_logs());

        // Auto choose.
        assert_eq!(battle.set_player_choice("player-2", "team"), Ok(()));
        // Not enough Mons, auto choose the rest.
        assert_eq!(battle.set_player_choice("player-3", "team 1 2"), Ok(()));
        // Reselect Mons.
        assert_eq!(battle.set_player_choice("player-3", "team 2 5"), Ok(()));
        // Too many Mons, truncate the list.
        assert_eq!(
            battle.set_player_choice("player-4", "team 5 4 3 2 1 0"),
            Ok(())
        );
        // No more active requests.
        assert!(battle.active_requests().collect::<Vec<_>>().is_empty());
        assert_eq!(battle.ready_to_continue(), Ok(true));
        assert_eq!(battle.continue_battle(), Ok(()));

        // New logs show updated team size and selected team leads.
        battle_io.verify_new_logs(&mut battle);

        battle_io.verify_next_request_set(&mut battle);

        // Turn 1: each player switches to Mon 1.
        assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-3", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-4", "switch 1"), Ok(()));

        assert!(battle.active_requests().collect::<Vec<_>>().is_empty());
        assert_eq!(battle.ready_to_continue(), Ok(true));
        assert_eq!(battle.continue_battle(), Ok(()));

        battle_io.verify_new_logs(&mut battle);

        battle_io.verify_next_request_set(&mut battle);

        // Turn 2: each player switches to Mon 2.
        assert_eq!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-3", "switch 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-4", "switch 2"), Ok(()));

        assert!(battle.active_requests().collect::<Vec<_>>().is_empty());
        assert_eq!(battle.ready_to_continue(), Ok(true));
        assert_eq!(battle.continue_battle(), Ok(()));

        battle_io.verify_new_logs(&mut battle);

        battle_io.verify_next_request_set(&mut battle);

        // Turn 3: each player tries to switch to Mon 3.
        assert_error_message(
            battle.set_player_choice("player-1", "switch 3"),
            "cannot switch: you do not have a Mon in slot 3 to switch to",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "switch 3"),
            "cannot switch: you do not have a Mon in slot 3 to switch to",
        );
        assert_error_message(
            battle.set_player_choice("player-3", "switch 3"),
            "cannot switch: you do not have a Mon in slot 3 to switch to",
        );
        assert_error_message(
            battle.set_player_choice("player-4", "switch 3"),
            "cannot switch: you do not have a Mon in slot 3 to switch to",
        );

        // Verify other slots fail for good measure.
        assert_error_message(
            battle.set_player_choice("player-1", "switch 4"),
            "cannot switch: you do not have a Mon in slot 4 to switch to",
        );
        assert_error_message(
            battle.set_player_choice("player-1", "switch 5"),
            "cannot switch: you do not have a Mon in slot 5 to switch to",
        );
        assert_error_message(
            battle.set_player_choice("player-1", "switch 6"),
            "cannot switch: you do not have a Mon in slot 6 to switch to",
        );

        // Switch back to Mon 0 (the lead that started the battle).
        assert_eq!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-3", "switch 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-4", "switch 0"), Ok(()));

        assert!(battle.active_requests().collect::<Vec<_>>().is_empty());
        assert_eq!(battle.ready_to_continue(), Ok(true));
        assert_eq!(battle.continue_battle(), Ok(()));

        battle_io.verify_new_logs(&mut battle);

        battle_io.verify_next_request_set(&mut battle);
    }
}
