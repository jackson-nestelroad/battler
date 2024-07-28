#[cfg(test)]
mod heal_test {
    use battler::{
        battle::{
            BattleType,
            CoreBattleEngineSpeedSortTieResolution,
            PublicCoreBattle,
        },
        common::Error,
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

    fn make_battle(
        data: &dyn DataStore,
        battle_type: BattleType,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(battle_type)
            .with_seed(124356453)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn heals_percent_of_user_hp() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "No Ability",
                        "moves": [
                            "Thunderbolt",
                            "Recover"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, BattleType::Singles, team.clone(), team).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Charizard,player-2,1|name:Thunderbolt|target:Charizard,player-1,1",
                "supereffective|mon:Charizard,player-1,1",
                "split|side:0",
                "damage|mon:Charizard,player-1,1|health:90/266",
                "damage|mon:Charizard,player-1,1|health:34/100",
                "move|mon:Charizard,player-1,1|name:Thunderbolt|target:Charizard,player-2,1",
                "supereffective|mon:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:84/266",
                "damage|mon:Charizard,player-2,1|health:32/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Recover|target:Charizard,player-2,1",
                "split|side:1",
                "heal|mon:Charizard,player-2,1|health:217/266",
                "heal|mon:Charizard,player-2,1|health:82/100",
                "move|mon:Charizard,player-1,1|name:Recover|target:Charizard,player-1,1",
                "split|side:0",
                "heal|mon:Charizard,player-1,1|health:223/266",
                "heal|mon:Charizard,player-1,1|health:84/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Recover|target:Charizard,player-2,1",
                "split|side:1",
                "heal|mon:Charizard,player-2,1|health:266/266",
                "heal|mon:Charizard,player-2,1|health:100/100",
                "move|mon:Charizard,player-1,1|name:Recover|target:Charizard,player-1,1",
                "split|side:0",
                "heal|mon:Charizard,player-1,1|health:266/266",
                "heal|mon:Charizard,player-1,1|health:100/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Recover|noanim",
                "fail|mon:Charizard,player-2,1|what:heal",
                "move|mon:Charizard,player-1,1|name:Recover|noanim",
                "fail|mon:Charizard,player-1,1|what:heal",
                "residual",
                "turn|turn:5"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn heals_all_allies() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Blastoise",
                        "species": "Blastoise",
                        "ability": "No Ability",
                        "moves": ["Life Dew"],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    },
                    {
                        "name": "Golem",
                        "species": "Golem",
                        "ability": "No Ability",
                        "moves": ["Earthquake"],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, BattleType::Doubles, team.clone(), team).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Golem,player-2,2|name:Earthquake|spread:Blastoise,player-2,1;Blastoise,player-1,1;Golem,player-1,2",
                "supereffective|mon:Golem,player-1,2",
                "split|side:1",
                "damage|mon:Blastoise,player-2,1|health:156/268",
                "damage|mon:Blastoise,player-2,1|health:59/100",
                "split|side:0",
                "damage|mon:Blastoise,player-1,1|health:163/268",
                "damage|mon:Blastoise,player-1,1|health:61/100",
                "split|side:0",
                "damage|mon:Golem,player-1,2|health:106/270",
                "damage|mon:Golem,player-1,2|health:40/100",
                "move|mon:Golem,player-1,2|name:Earthquake|spread:Blastoise,player-1,1;Blastoise,player-2,1;Golem,player-2,2",
                "supereffective|mon:Golem,player-2,2",
                "split|side:0",
                "damage|mon:Blastoise,player-1,1|health:55/268",
                "damage|mon:Blastoise,player-1,1|health:21/100",
                "split|side:1",
                "damage|mon:Blastoise,player-2,1|health:53/268",
                "damage|mon:Blastoise,player-2,1|health:20/100",
                "split|side:1",
                "damage|mon:Golem,player-2,2|health:112/270",
                "damage|mon:Golem,player-2,2|health:42/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Blastoise,player-2,1|name:Life Dew|spread:Golem,player-2,2;Blastoise,player-2,1",
                "split|side:1",
                "heal|mon:Golem,player-2,2|health:179/270",
                "heal|mon:Golem,player-2,2|health:67/100",
                "split|side:1",
                "heal|mon:Blastoise,player-2,1|health:120/268",
                "heal|mon:Blastoise,player-2,1|health:45/100",
                "move|mon:Blastoise,player-1,1|name:Life Dew|spread:Golem,player-1,2;Blastoise,player-1,1",
                "split|side:0",
                "heal|mon:Golem,player-1,2|health:173/270",
                "heal|mon:Golem,player-1,2|health:65/100",
                "split|side:0",
                "heal|mon:Blastoise,player-1,1|health:122/268",
                "heal|mon:Blastoise,player-1,1|health:46/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
