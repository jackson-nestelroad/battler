#[cfg(test)]
mod switch_bad_input_tests {
    use battler::{
        battle::{
            Battle,
            BattleType,
            PublicCoreBattle,
        },
        common::{
            Error,
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
        assert_error_message_contains,
        TestBattleBuilder,
    };

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Chlorophyll",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Chlorophyll",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 1
                    },
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Chlorophyll",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 1
                    },
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Chlorophyll",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 1
                    },
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Chlorophyll",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 1
                    },
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Chlorophyll",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 1
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team()?)
            .with_team("player-2", team()?)
            .build(data)
    }

    #[test]
    fn too_many_switches() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_error_message(
            battle.set_player_choice("player-1", "switch 2; switch 3; switch 4"),
            "cannot switch: you sent more choices than active Mons",
        );
    }

    #[test]
    fn missing_position() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_error_message(
            battle.set_player_choice("player-1", "switch"),
            "cannot switch: you must select a Mon to switch in",
        );
        assert_error_message(
            battle.set_player_choice("player-1", "switch  "),
            "cannot switch: you must select a Mon to switch in",
        );
    }

    #[test]
    fn invalid_position() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_error_message_contains(
            battle.set_player_choice("player-1", "switch Charmander"),
            "cannot switch: switch argument is not an integer",
        );
        assert_error_message_contains(
            battle.set_player_choice("player-1", "switch -1"),
            "cannot switch: switch argument is not an integer",
        );
    }

    #[test]
    fn no_mon_in_position() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_error_message(
            battle.set_player_choice("player-1", "switch 6"),
            "cannot switch: you do not have a Mon in slot 6 to switch to",
        );
        assert_error_message(
            battle.set_player_choice("player-1", "switch 10"),
            "cannot switch: you do not have a Mon in slot 10 to switch to",
        );
    }

    #[test]
    fn switch_to_active_mon() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_error_message(
            battle.set_player_choice("player-1", "switch 0"),
            "cannot switch: you cannot switch to an active Mon",
        );
        assert_error_message(
            battle.set_player_choice("player-1", "switch 1"),
            "cannot switch: you cannot switch to an active Mon",
        );
        assert_error_message(
            battle.set_player_choice("player-1", "switch 2; switch 1"),
            "cannot switch: you cannot switch to an active Mon",
        );
    }

    #[test]
    fn switch_a_mon_in_twice() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_error_message(
            battle.set_player_choice("player-1", "switch 2; switch 2"),
            "cannot switch: the Mon in slot 2 can only switch in once",
        );
    }

    #[test]
    fn switch_to_fainted_mon() {
        // TODO: Force Mon to faint then try to switch to it.
    }

    #[test]
    fn switch_out_trapped_mon() {
        // TODO: Attempt to switch out a trapped Mon.
    }

    #[test]
    fn too_many_force_switches() {
        // TODO: Force a switch and send too many choices.
    }
}
