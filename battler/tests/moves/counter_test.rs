#[cfg(test)]
mod counter_test {
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
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "No Ability",
                        "moves": [
                            "Fly",
                            "Flamethrower"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Alakazam",
                        "species": "Alakazam",
                        "ability": "No Ability",
                        "moves": [
                            "Counter",
                            "Tackle"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
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
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn counter_doubles_damage_of_last_physical_hit_on_user() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 1000, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "move 0,2;move 0"),
            Ok(())
        );

        assert_eq!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "move 0;move 1,2"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "switch|player:player-1|position:2|name:Alakazam|health:100/100|species:Alakazam|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "switch|player:player-2|position:2|name:Alakazam|health:100/100|species:Alakazam|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Fly|noanim",
                "prepare|mon:Charizard,player-2,1|move:Fly",
                "move|mon:Alakazam,player-2,2|name:Counter|noanim",
                "fail|mon:Alakazam,player-2,2",
                "move|mon:Alakazam,player-1,2|name:Counter|noanim",
                "fail|mon:Alakazam,player-1,2",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Alakazam,player-2,2|name:Tackle|target:Alakazam,player-1,2",
                "split|side:0",
                "damage|mon:Alakazam,player-1,2|health:96/115",
                "damage|mon:Alakazam,player-1,2|health:84/100",
                "move|mon:Charizard,player-2,1|name:Fly|target:Alakazam,player-1,2",
                "split|side:0",
                "damage|mon:Alakazam,player-1,2|health:3/115",
                "damage|mon:Alakazam,player-1,2|health:3/100",
                "move|mon:Alakazam,player-1,2|name:Counter|target:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:0",
                "damage|mon:Charizard,player-2,1|health:0",
                "faint|mon:Charizard,player-2,1",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn counter_does_not_counter_special_damage() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 1000, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "move 1,2;move 0"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "switch|player:player-1|position:2|name:Alakazam|health:100/100|species:Alakazam|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "switch|player:player-2|position:2|name:Alakazam|health:100/100|species:Alakazam|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Flamethrower|target:Alakazam,player-1,2",
                "split|side:0",
                "damage|mon:Alakazam,player-1,2|health:48/115",
                "damage|mon:Alakazam,player-1,2|health:42/100",
                "move|mon:Alakazam,player-2,2|name:Counter|noanim",
                "fail|mon:Alakazam,player-2,2",
                "move|mon:Alakazam,player-1,2|name:Counter|noanim",
                "fail|mon:Alakazam,player-1,2",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
