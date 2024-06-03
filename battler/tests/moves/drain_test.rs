#[cfg(test)]
mod drain_test {
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
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(0)
            .with_team_validation(false)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn drain_moves_heal_a_percent_of_damage_dealt() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team_1: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "No Ability",
                        "moves": [
                            "Mega Drain",
                            "Giga Drain"
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
        let team_2: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Machamp",
                        "species": "Machamp",
                        "ability": "No Ability",
                        "moves": [
                            "Vital Throw"
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

        let mut battle = make_battle(&data, team_1, team_2).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:M",
                "switch|player:player-2|position:1|name:Machamp|health:100/100|species:Machamp|level:100|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Mega Drain|target:Machamp,player-2,1",
                "split|side:1",
                "damage|mon:Machamp,player-2,1|health:232/290",
                "damage|mon:Machamp,player-2,1|health:80/100",
                "move|mon:Machamp,player-2,1|name:Vital Throw|target:Venusaur,player-1,1",
                "resisted|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:207/270",
                "damage|mon:Venusaur,player-1,1|health:77/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Machamp,player-2,1",
                "split|side:1",
                "damage|mon:Machamp,player-2,1|health:129/290",
                "damage|mon:Machamp,player-2,1|health:45/100",
                "split|side:0",
                "heal|mon:Venusaur,player-1,1|from:Drain|of:Machamp,player-2,1|health:259/270",
                "heal|mon:Venusaur,player-1,1|from:Drain|of:Machamp,player-2,1|health:96/100",
                "move|mon:Machamp,player-2,1|name:Vital Throw|target:Venusaur,player-1,1",
                "resisted|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:196/270",
                "damage|mon:Venusaur,player-1,1|health:73/100",
                "residual",
                "turn|turn:3"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
