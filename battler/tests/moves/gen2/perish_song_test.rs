#[cfg(test)]
mod perish_song_test {
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
        assert_logs_since_turn_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Misdreavus",
                        "species": "Misdreavus",
                        "ability": "No Ability",
                        "moves": [
                            "Perish Song"
                        ],
                        "nature": "Hardy",
                        "level": 50
                    },
                    {
                        "name": "Smoochum",
                        "species": "Smoochum",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "level": 50
                    },
                    {
                        "name": "Whismur",
                        "species": "Whismur",
                        "ability": "Soundproof",
                        "moves": [],
                        "nature": "Hardy",
                        "level": 50
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn make_battle(
        data: &dyn DataStore,
        seed: u64,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .with_seed(seed)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Reverse)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn perish_song_faints_all_active_mons() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-1", "switch 2;pass"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Misdreavus,player-1,1|name:Perish Song",
                "fieldactivate|move:Perish Song",
                "move|mon:Misdreavus,player-2,1|name:Perish Song",
                "start|mon:Smoochum,player-2,2|move:Perish Song|perish:3",
                "start|mon:Misdreavus,player-2,1|move:Perish Song|perish:3",
                "start|mon:Smoochum,player-1,2|move:Perish Song|perish:3",
                "start|mon:Misdreavus,player-1,1|move:Perish Song|perish:3",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-1|position:1|name:Whismur|health:100/100|species:Whismur|level:50|gender:F",
                "start|mon:Smoochum,player-2,2|move:Perish Song|perish:2",
                "start|mon:Misdreavus,player-2,1|move:Perish Song|perish:2",
                "start|mon:Smoochum,player-1,2|move:Perish Song|perish:2",
                "residual",
                "turn|turn:3",
                ["time"],
                "start|mon:Smoochum,player-2,2|move:Perish Song|perish:1",
                "start|mon:Misdreavus,player-2,1|move:Perish Song|perish:1",
                "start|mon:Smoochum,player-1,2|move:Perish Song|perish:1",
                "residual",
                "turn|turn:4",
                ["time"],
                "start|mon:Smoochum,player-2,2|move:Perish Song|perish:0",
                "start|mon:Misdreavus,player-2,1|move:Perish Song|perish:0",
                "start|mon:Smoochum,player-1,2|move:Perish Song|perish:0",
                "residual",
                "faint|mon:Smoochum,player-2,2",
                "faint|mon:Misdreavus,player-2,1",
                "faint|mon:Smoochum,player-1,2"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn soundproof_resists_perish_song() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "switch 2;pass"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "switch|player:player-1|position:1|name:Whismur|health:100/100|species:Whismur|level:50|gender:F",
                "move|mon:Misdreavus,player-2,1|name:Perish Song",
                "immune|mon:Whismur,player-1,1|from:ability:Soundproof",
                "fieldactivate|move:Perish Song",
                "start|mon:Smoochum,player-2,2|move:Perish Song|perish:3",
                "start|mon:Misdreavus,player-2,1|move:Perish Song|perish:3",
                "start|mon:Smoochum,player-1,2|move:Perish Song|perish:3",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
