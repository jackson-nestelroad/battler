#[cfg(test)]
mod multihit_test {
    use battler::{
        battle::{
            Battle,
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
        assert_turn_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn make_team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Overgrow",
                        "moves": [
                            "Fury Attack",
                            "Double Kick",
                            "Icicle Spear"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Charmander",
                        "species": "Charmander",
                        "ability": "Blaze",
                        "moves": [],
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

    fn make_battle(data: &dyn DataStore, seed: u64) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(seed)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", make_team()?)
            .with_team("player-2", make_team()?)
            .build(data)
    }

    #[test]
    fn multihit_number_in_range() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 8888888124).unwrap();
        assert_eq!(battle.start(), Ok(()));
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
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:99/105",
                "damage|mon:Bulbasaur,player-2,1|health:95/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:92/105",
                "damage|mon:Bulbasaur,player-2,1|health:88/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:85/105",
                "damage|mon:Bulbasaur,player-2,1|health:81/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:78/105",
                "damage|mon:Bulbasaur,player-2,1|health:75/100",
                "hitcount|hits:4",
                "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:98/105",
                "damage|mon:Bulbasaur,player-1,1|health:94/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:91/105",
                "damage|mon:Bulbasaur,player-1,1|health:87/100",
                "hitcount|hits:2",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:84/105",
                "damage|mon:Bulbasaur,player-1,1|health:80/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:77/105",
                "damage|mon:Bulbasaur,player-1,1|health:74/100",
                "hitcount|hits:2",
                "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
                "crit|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:68/105",
                "damage|mon:Bulbasaur,player-2,1|health:65/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:61/105",
                "damage|mon:Bulbasaur,player-2,1|health:59/100",
                "crit|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:51/105",
                "damage|mon:Bulbasaur,player-2,1|health:49/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:44/105",
                "damage|mon:Bulbasaur,player-2,1|health:42/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:37/105",
                "damage|mon:Bulbasaur,player-2,1|health:36/100",
                "hitcount|hits:5",
                "residual",
                "turn|turn:3"
            ]"#,
        ).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn multihit_number_in_range_with_crits() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 8888888123).unwrap();
        assert_eq!(battle.start(), Ok(()));
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
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:98/105",
                "damage|mon:Bulbasaur,player-2,1|health:94/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:91/105",
                "damage|mon:Bulbasaur,player-2,1|health:87/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:84/105",
                "damage|mon:Bulbasaur,player-2,1|health:80/100",
                "hitcount|hits:3",
                "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:98/105",
                "damage|mon:Bulbasaur,player-1,1|health:94/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:91/105",
                "damage|mon:Bulbasaur,player-1,1|health:87/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:84/105",
                "damage|mon:Bulbasaur,player-1,1|health:80/100",
                "hitcount|hits:3",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:77/105",
                "damage|mon:Bulbasaur,player-1,1|health:74/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:70/105",
                "damage|mon:Bulbasaur,player-1,1|health:67/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:64/105",
                "damage|mon:Bulbasaur,player-1,1|health:61/100",
                "hitcount|hits:3",
                "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:77/105",
                "damage|mon:Bulbasaur,player-2,1|health:74/100",
                "crit|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:66/105",
                "damage|mon:Bulbasaur,player-2,1|health:63/100",
                "crit|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:55/105",
                "damage|mon:Bulbasaur,player-2,1|health:53/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:48/105",
                "damage|mon:Bulbasaur,player-2,1|health:46/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:42/105",
                "damage|mon:Bulbasaur,player-2,1|health:40/100",
                "hitcount|hits:5",
                "residual",
                "turn|turn:3"
            ]"#,
        ).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn multihit_static_number() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 8888888123).unwrap();
        assert_eq!(battle.start(), Ok(()));
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
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-1,1|name:Double Kick|target:Bulbasaur,player-2,1",
                "resisted|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:99/105",
                "damage|mon:Bulbasaur,player-2,1|health:95/100",
                "resisted|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:93/105",
                "damage|mon:Bulbasaur,player-2,1|health:89/100",
                "hitcount|hits:2",
                "move|mon:Bulbasaur,player-2,1|name:Double Kick|target:Bulbasaur,player-1,1",
                "resisted|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:99/105",
                "damage|mon:Bulbasaur,player-1,1|health:95/100",
                "resisted|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:93/105",
                "damage|mon:Bulbasaur,player-1,1|health:89/100",
                "hitcount|hits:2",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Bulbasaur,player-1,1|name:Double Kick|target:Bulbasaur,player-2,1",
                "resisted|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:87/105",
                "damage|mon:Bulbasaur,player-2,1|health:83/100",
                "resisted|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:81/105",
                "damage|mon:Bulbasaur,player-2,1|health:78/100",
                "hitcount|hits:2",
                "move|mon:Bulbasaur,player-2,1|name:Double Kick|target:Bulbasaur,player-1,1",
                "resisted|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:87/105",
                "damage|mon:Bulbasaur,player-1,1|health:83/100",
                "resisted|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:81/105",
                "damage|mon:Bulbasaur,player-1,1|health:78/100",
                "hitcount|hits:2",
                "residual",
                "turn|turn:3"
            ]"#,
        ).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn hit_count_logs_after_faint() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Bulbasaur,player-1,1|name:Icicle Spear|target:Bulbasaur,player-2,1",
                "supereffective|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:19/105",
                "damage|mon:Bulbasaur,player-2,1|health:19/100",
                "supereffective|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:0",
                "damage|mon:Bulbasaur,player-2,1|health:0",
                "faint|mon:Bulbasaur,player-2,1",
                "hitcount|hits:2",
                "residual"
            ]"#,
        )
        .unwrap();
        assert_turn_logs_eq(&mut battle, 2, &expected_logs);
    }
}
