#[cfg(test)]
mod switch_out_state_test {
    use assert_matches::assert_matches;
    use battler::{
        battle::{
            Battle,
            BattleEngineSpeedSortTieResolution,
            BattleType,
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
        assert_new_logs_eq,
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
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
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
                "damage|mon:Bulbasaur,player-2,1|health:87/105",
                "damage|mon:Bulbasaur,player-2,1|health:83/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "switch|player:player-2|position:1|name:Charmander|health:80/100|species:Charmander|level:50|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Charmander,player-2,1",
                "split|side:1",
                "damage|mon:Charmander,player-2,1|health:60/99",
                "damage|mon:Charmander,player-2,1|health:61/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "switch|player:player-2|position:1|name:Bulbasaur|health:83/100|species:Bulbasaur|level:50|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:69/105",
                "damage|mon:Bulbasaur,player-2,1|health:66/100",
                "residual",
                "turn|turn:5"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        assert_matches!(battle.request_for_player("player-2"), Some(Request::Turn(request)) => {
            assert_eq!(request.player.mons[0].health, "69/105");
            assert_eq!(request.player.mons[1].health, "60/99");
        });
    }
}
