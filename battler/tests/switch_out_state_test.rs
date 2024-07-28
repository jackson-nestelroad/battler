#[cfg(test)]
mod switch_out_state_test {
    use assert_matches::assert_matches;
    use battler::{
        battle::{
            BattleType,
            CoreBattleEngineSpeedSortTieResolution,
            PublicCoreBattle,
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
        assert_logs_since_turn_eq,
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
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
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
    fn switch_out_preserves_health() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 0"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:50|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Charmander,player-2,1",
                "split|side:1",
                "damage|mon:Charmander,player-2,1|health:79/99",
                "damage|mon:Charmander,player-2,1|health:80/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:88/105",
                "damage|mon:Bulbasaur,player-2,1|health:84/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "switch|player:player-2|position:1|name:Charmander|health:80/100|species:Charmander|level:50|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Charmander,player-2,1",
                "split|side:1",
                "damage|mon:Charmander,player-2,1|health:62/99",
                "damage|mon:Charmander,player-2,1|health:63/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "switch|player:player-2|position:1|name:Bulbasaur|health:84/100|species:Bulbasaur|level:50|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:71/105",
                "damage|mon:Bulbasaur,player-2,1|health:68/100",
                "residual",
                "turn|turn:5"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&mut battle, 1, &expected_logs);

        assert_matches!(battle.request_for_player("player-2"), Some(Request::Turn(request)) => {
            assert_eq!(request.player.mons[0].health, "71/105");
            assert_eq!(request.player.mons[1].health, "62/99");
        });
    }
}
