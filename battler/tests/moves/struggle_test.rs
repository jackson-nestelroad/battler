#[cfg(test)]
mod struggle_test {
    use battler::{
        battle::{
            Battle,
            BattleEngineSpeedSortTieResolution,
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
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn struggle_deals_recoil() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Blissey",
                        "species": "Blissey",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, team.clone(), team).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
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
                "switch|player:player-1|position:1|name:Blissey|health:100/100|species:Blissey|level:100|gender:M",
                "switch|player:player-2|position:1|name:Blissey|health:100/100|species:Blissey|level:100|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Blissey,player-2,1|name:Struggle|target:Blissey,player-1,1",
                "crit|mon:Blissey,player-1,1",
                "split|side:0",
                "damage|mon:Blissey,player-1,1|health:562/620",
                "damage|mon:Blissey,player-1,1|health:91/100",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|from:Struggle Recoil|health:465/620",
                "damage|mon:Blissey,player-2,1|from:Struggle Recoil|health:75/100",
                "move|mon:Blissey,player-1,1|name:Struggle|target:Blissey,player-2,1",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|health:423/620",
                "damage|mon:Blissey,player-2,1|health:69/100",
                "split|side:0",
                "damage|mon:Blissey,player-1,1|from:Struggle Recoil|health:407/620",
                "damage|mon:Blissey,player-1,1|from:Struggle Recoil|health:66/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Blissey,player-2,1|name:Struggle|target:Blissey,player-1,1",
                "split|side:0",
                "damage|mon:Blissey,player-1,1|health:368/620",
                "damage|mon:Blissey,player-1,1|health:60/100",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|from:Struggle Recoil|health:268/620",
                "damage|mon:Blissey,player-2,1|from:Struggle Recoil|health:44/100",
                "move|mon:Blissey,player-1,1|name:Struggle|target:Blissey,player-2,1",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|health:227/620",
                "damage|mon:Blissey,player-2,1|health:37/100",
                "split|side:0",
                "damage|mon:Blissey,player-1,1|from:Struggle Recoil|health:213/620",
                "damage|mon:Blissey,player-1,1|from:Struggle Recoil|health:35/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Blissey,player-2,1|name:Struggle|target:Blissey,player-1,1",
                "split|side:0",
                "damage|mon:Blissey,player-1,1|health:170/620",
                "damage|mon:Blissey,player-1,1|health:28/100",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|from:Struggle Recoil|health:72/620",
                "damage|mon:Blissey,player-2,1|from:Struggle Recoil|health:12/100",
                "move|mon:Blissey,player-1,1|name:Struggle|target:Blissey,player-2,1",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|health:32/620",
                "damage|mon:Blissey,player-2,1|health:6/100",
                "split|side:0",
                "damage|mon:Blissey,player-1,1|from:Struggle Recoil|health:15/620",
                "damage|mon:Blissey,player-1,1|from:Struggle Recoil|health:3/100",
                "residual",
                "turn|turn:4"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn struggle_is_typeless() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Gengar",
                        "species": "Gengar",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, team.clone(), team).unwrap();

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
                "switch|player:player-1|position:1|name:Gengar|health:100/100|species:Gengar|level:100|gender:M",
                "switch|player:player-2|position:1|name:Gengar|health:100/100|species:Gengar|level:100|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Gengar,player-2,1|name:Struggle|target:Gengar,player-1,1",
                "crit|mon:Gengar,player-1,1",
                "split|side:0",
                "damage|mon:Gengar,player-1,1|health:169/230",
                "damage|mon:Gengar,player-1,1|health:74/100",
                "split|side:1",
                "damage|mon:Gengar,player-2,1|from:Struggle Recoil|health:172/230",
                "damage|mon:Gengar,player-2,1|from:Struggle Recoil|health:75/100",
                "move|mon:Gengar,player-1,1|name:Struggle|target:Gengar,player-2,1",
                "split|side:1",
                "damage|mon:Gengar,player-2,1|health:127/230",
                "damage|mon:Gengar,player-2,1|health:56/100",
                "split|side:0",
                "damage|mon:Gengar,player-1,1|from:Struggle Recoil|health:111/230",
                "damage|mon:Gengar,player-1,1|from:Struggle Recoil|health:49/100",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
