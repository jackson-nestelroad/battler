#[cfg(test)]
mod heal_test {
    use battler::{
        battle::{
            Battle,
            BattleType,
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
        assert_new_logs_eq,
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
            .with_seed(0)
            .with_team_validation(false)
            .with_pass_allowed(true)
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
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Thunderbolt|target:Charizard,player-1,1",
                "supereffective|mon:Charizard,player-1,1",
                "split|side:0",
                "damage|mon:Charizard,player-1,1|health:70/266",
                "damage|mon:Charizard,player-1,1|health:27/100",
                "move|mon:Charizard,player-1,1|name:Thunderbolt|target:Charizard,player-2,1",
                "supereffective|mon:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:72/266",
                "damage|mon:Charizard,player-2,1|health:28/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Charizard,player-1,1|name:Recover|target:Charizard,player-1,1",
                "split|side:0",
                "heal|mon:Charizard,player-1,1|health:203/266",
                "heal|mon:Charizard,player-1,1|health:77/100",
                "move|mon:Charizard,player-2,1|name:Recover|target:Charizard,player-2,1",
                "split|side:1",
                "heal|mon:Charizard,player-2,1|health:205/266",
                "heal|mon:Charizard,player-2,1|health:78/100",
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
                "move|mon:Charizard,player-2,1|name:Recover|target:Charizard,player-2,1",
                "fail|mon:Charizard,player-2,1|what:heal",
                "move|mon:Charizard,player-1,1|name:Recover|target:Charizard,player-1,1",
                "fail|mon:Charizard,player-1,1|what:heal",
                "residual",
                "turn|turn:5"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
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
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Blastoise|health:100/100|species:Blastoise|level:100|gender:M",
                "switch|player:player-1|position:2|name:Golem|health:100/100|species:Golem|level:100|gender:M",
                "switch|player:player-2|position:1|name:Blastoise|health:100/100|species:Blastoise|level:100|gender:M",
                "switch|player:player-2|position:2|name:Golem|health:100/100|species:Golem|level:100|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Golem,player-2,2|name:Earthquake|spread:Blastoise,player-2,1;Blastoise,player-1,1;Golem,player-1,2",
                "supereffective|mon:Golem,player-1,2",
                "split|side:1",
                "damage|mon:Blastoise,player-2,1|health:166/268",
                "damage|mon:Blastoise,player-2,1|health:62/100",
                "split|side:0",
                "damage|mon:Blastoise,player-1,1|health:160/268",
                "damage|mon:Blastoise,player-1,1|health:60/100",
                "split|side:0",
                "damage|mon:Golem,player-1,2|health:96/270",
                "damage|mon:Golem,player-1,2|health:36/100",
                "move|mon:Golem,player-1,2|name:Earthquake|spread:Blastoise,player-1,1;Blastoise,player-2,1;Golem,player-2,2",
                "supereffective|mon:Golem,player-2,2",
                "split|side:0",
                "damage|mon:Blastoise,player-1,1|health:60/268",
                "damage|mon:Blastoise,player-1,1|health:23/100",
                "split|side:1",
                "damage|mon:Blastoise,player-2,1|health:57/268",
                "damage|mon:Blastoise,player-2,1|health:22/100",
                "split|side:1",
                "damage|mon:Golem,player-2,2|health:120/270",
                "damage|mon:Golem,player-2,2|health:45/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Blastoise,player-2,1|name:Life Dew|spread:Golem,player-2,2;Blastoise,player-2,1",
                "split|side:1",
                "heal|mon:Golem,player-2,2|health:188/270",
                "heal|mon:Golem,player-2,2|health:70/100",
                "split|side:1",
                "heal|mon:Blastoise,player-2,1|health:124/268",
                "heal|mon:Blastoise,player-2,1|health:47/100",
                "move|mon:Blastoise,player-1,1|name:Life Dew|spread:Golem,player-1,2;Blastoise,player-1,1",
                "split|side:0",
                "heal|mon:Golem,player-1,2|health:164/270",
                "heal|mon:Golem,player-1,2|health:61/100",
                "split|side:0",
                "heal|mon:Blastoise,player-1,1|health:127/268",
                "heal|mon:Blastoise,player-1,1|health:48/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
