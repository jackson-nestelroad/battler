#[cfg(test)]
mod switch_before_move_test {
    use battler::{
        battle::{
            Battle,
            BattleEngineSpeedSortTieResolution,
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
        assert_turn_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Overgrow",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Charmander",
                        "species": "Charmander",
                        "ability": "Blaze",
                        "moves": ["Scratch"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Squirtle",
                        "species": "Squirtle",
                        "ability": "Torrent",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn make_battle_builder() -> TestBattleBuilder {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(0)
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
    }

    fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        make_battle_builder()
            .with_team("player-1", team()?)
            .with_team("player-2", team()?)
            .build(data)
    }

    #[test]
    fn move_hits_switched_in_mon() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 0"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:50|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Charmander,player-2,1",
                "split|side:1",
                ["damage|mon:Charmander,player-2,1"],
                ["damage|mon:Charmander,player-2,1"],
                "residual"
            ]"#,
        )
        .unwrap();
        assert_turn_logs_eq(&mut battle, 1, &expected_logs);

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "split|side:1",
                ["damage|mon:Bulbasaur,player-2,1"],
                ["damage|mon:Bulbasaur,player-2,1"],
                "residual"
            ]"#,
        )
        .unwrap();
        assert_turn_logs_eq(&mut battle, 2, &expected_logs);
    }
}
