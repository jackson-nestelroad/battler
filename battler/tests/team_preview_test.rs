#[cfg(test)]
mod team_preview_tests {
    use battler::{
        battle::{
            Battle,
            BattleType,
            CoreBattle,
            Request,
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
        assert_new_logs_eq,
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

    fn make_multi_battle(data: &dyn DataStore) -> Result<CoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Multi)
            .with_rule("Standard")
            .with_rule("! Species Clause")
            .with_rule("Force Level = 100")
            .with_rule("Min Team Size = 3")
            .with_rule("Picked Team Size = 3")
            .with_rule("Team Preview")
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
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_multi_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));
        pretty_assertions::assert_eq!(
            battle.active_requests().collect::<Vec<_>>(),
            vec![
                ("player-1".to_owned(), Request::TeamPreview),
                ("player-2".to_owned(), Request::TeamPreview),
                ("player-3".to_owned(), Request::TeamPreview),
                ("player-4".to_owned(), Request::TeamPreview),
            ]
        );
        assert_eq!(battle.ready_to_continue(), Ok(false));
        assert_new_logs_eq(
            &mut battle,
            &[
                "battletype|Multi",
                "rule|Endless Battle Clause: Forcing endless battles is banned",
                "rule|Sleep Clause: Limit one foe put to sleep",
                "player|player-1|0|0",
                "player|player-2|0|1",
                "player|player-3|1|0",
                "player|player-4|1|1",
                "teamsize|player-1|6",
                "teamsize|player-2|6",
                "teamsize|player-3|6",
                "teamsize|player-4|6",
                "teampreviewstart",
                "mon|player-1|Bulbasaur|100|F",
                "mon|player-1|Charmander|100|F",
                "mon|player-1|Squirtle|100|F",
                "mon|player-1|Bulbasaur|100|M",
                "mon|player-1|Charmander|100|M",
                "mon|player-1|Squirtle|100|M",
                "mon|player-2|Bulbasaur|100|F",
                "mon|player-2|Charmander|100|F",
                "mon|player-2|Squirtle|100|F",
                "mon|player-2|Bulbasaur|100|M",
                "mon|player-2|Charmander|100|M",
                "mon|player-2|Squirtle|100|M",
                "mon|player-3|Bulbasaur|100|F",
                "mon|player-3|Charmander|100|F",
                "mon|player-3|Squirtle|100|F",
                "mon|player-3|Bulbasaur|100|M",
                "mon|player-3|Charmander|100|M",
                "mon|player-3|Squirtle|100|M",
                "mon|player-4|Bulbasaur|100|F",
                "mon|player-4|Charmander|100|F",
                "mon|player-4|Squirtle|100|F",
                "mon|player-4|Bulbasaur|100|M",
                "mon|player-4|Charmander|100|M",
                "mon|player-4|Squirtle|100|M",
                "teampreview|3",
            ],
        );
        // Player 1 made their choice.
        assert_eq!(battle.set_player_choice("player-1", "team 0 1 2"), Ok(()));
        pretty_assertions::assert_eq!(
            battle.active_requests().collect::<Vec<_>>(),
            vec![
                ("player-2".to_owned(), Request::TeamPreview),
                ("player-3".to_owned(), Request::TeamPreview),
                ("player-4".to_owned(), Request::TeamPreview),
            ]
        );
        assert_eq!(battle.ready_to_continue(), Ok(false));
        assert!(!battle.has_new_logs());

        // Auto choose.
        assert_eq!(battle.set_player_choice("player-2", "team"), Ok(()));
        // Not enough Mons, auto choose the rest.
        assert_eq!(battle.set_player_choice("player-3", "team 1 2"), Ok(()));
        // Reselect Mons.
        assert_eq!(battle.set_player_choice("player-3", "team 2 1"), Ok(()));
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
        assert_new_logs_eq(
            &mut battle,
            &[
                "",
                "teamsize|player-1|3",
                "teamsize|player-2|3",
                "teamsize|player-3|3",
                "teamsize|player-4|3",
                "start",
                "switch|player-1|0|Bulbasaur F|Bulbasaur|100|F",
                "switch|player-2|0|Bulbasaur F|Bulbasaur|100|F",
                "switch|player-3|0|Squirtle F|Squirtle|100|F",
                "switch|player-4|0|Squirtle M|Squirtle|100|M",
                "turn|1",
            ],
        );

        pretty_assertions::assert_eq!(
            battle.active_requests().collect::<Vec<_>>(),
            vec![
                ("player-1".to_owned(), Request::Turn),
                ("player-2".to_owned(), Request::Turn),
                ("player-3".to_owned(), Request::Turn),
                ("player-4".to_owned(), Request::Turn),
            ]
        );

        // TODO: Implement switching.
        // TODO: Turn 1: switch to Mon 1 for all players.
        // TODO: Turn 2: switch to Mon 2 for all players.
        // TODO: Turn 3: attempt to switch to Mon 3 for all players (should fail).
    }
}
