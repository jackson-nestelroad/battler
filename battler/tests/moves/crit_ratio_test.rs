#[cfg(test)]
mod crit_ratio_test {
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
        team_1: TeamData,
        team_2: TeamData,
        seed: u64,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(seed)
            .with_team_validation(false)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn increased_crit_ratio() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Overgrow",
                        "moves": [
                            "Razor Leaf"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, team.clone(), team, 12914159537808871000).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
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
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Razor Leaf",
                "resisted|mon:Bulbasaur,player-1,1",
                "crit|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:94/105",
                "damage|mon:Bulbasaur,player-1,1|health:90/100",
                "move|mon:Bulbasaur,player-1,1|name:Razor Leaf",
                "resisted|mon:Bulbasaur,player-2,1",
                "crit|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:92/105",
                "damage|mon:Bulbasaur,player-2,1|health:88/100",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn moves_can_force_crit() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Overgrow",
                        "moves": [
                            "Frost Breath"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, team.clone(), team, 1).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
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
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-1,1|name:Frost Breath|target:Bulbasaur,player-2,1",
                "supereffective|mon:Bulbasaur,player-2,1",
                "crit|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:23/105",
                "damage|mon:Bulbasaur,player-2,1|health:22/100",
                "move|mon:Bulbasaur,player-2,1|name:Frost Breath|target:Bulbasaur,player-1,1",
                "supereffective|mon:Bulbasaur,player-1,1",
                "crit|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:27/105",
                "damage|mon:Bulbasaur,player-1,1|health:26/100",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
