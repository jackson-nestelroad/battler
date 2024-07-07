#[cfg(test)]
mod no_target_test {
    use battler::{
        battle::{
            BattleType,
            CoreBattleEngineSpeedSortTieResolution,
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
                        "moves": ["Tackle", "Air Cutter"],
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
                        "level": 5
                    },
                    {
                        "name": "Squirtle",
                        "species": "Squirtle",
                        "ability": "Torrent",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 5
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn make_battle_builder() -> TestBattleBuilder {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .with_seed(0)
            .with_pass_allowed(true)
            .with_team_validation(false)
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
    fn retargets_foe_after_original_target_faints() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "move 0,2;move 0,2"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Charmander|health:100/100|species:Charmander|level:5|gender:F",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Charmander|health:100/100|species:Charmander|level:5|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Charmander,player-2,2",
                "split|side:1",
                "damage|mon:Charmander,player-2,2|health:0",
                "damage|mon:Charmander,player-2,2|health:0",
                "faint|mon:Charmander,player-2,2",
                "move|mon:Charmander,player-1,2|name:Scratch|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:104/105",
                "damage|mon:Bulbasaur,player-2,1|health:99/100",
                "residual"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn move_fails_with_no_target() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "switch 2;pass"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-1", "move 1;move 0,2"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Charmander|health:100/100|species:Charmander|level:5|gender:F",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Charmander|health:100/100|species:Charmander|level:5|gender:F",
                "turn|turn:1",
                ["time"],
                "switch|player:player-2|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:F",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Bulbasaur,player-1,1|name:Air Cutter|spread:Squirtle,player-2,1;Charmander,player-2,2",
                "crit|mon:Charmander,player-2,2",
                "split|side:1",
                "damage|mon:Squirtle,player-2,1|health:0",
                "damage|mon:Squirtle,player-2,1|health:0",
                "split|side:1",
                "damage|mon:Charmander,player-2,2|health:0",
                "damage|mon:Charmander,player-2,2|health:0",
                "faint|mon:Squirtle,player-2,1",
                "faint|mon:Charmander,player-2,2",
                "move|mon:Charmander,player-1,2|name:Scratch|notarget",
                "fail|mon:Charmander,player-1,2",
                "residual"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
