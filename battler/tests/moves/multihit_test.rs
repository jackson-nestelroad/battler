#[cfg(test)]
mod multihit_test {
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
                            "Icicle Spear",
                            "Twineedle"
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
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", make_team()?)
            .with_team("player-2", make_team()?)
            .build(data)
    }

    #[test]
    fn multihit_number_in_range() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 766108902979015).unwrap();
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
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:99/105",
                "damage|mon:Bulbasaur,player-1,1|health:95/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:92/105",
                "damage|mon:Bulbasaur,player-1,1|health:88/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:85/105",
                "damage|mon:Bulbasaur,player-1,1|health:81/100",
                "hitcount|hits:3",
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
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:78/105",
                "damage|mon:Bulbasaur,player-1,1|health:75/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:71/105",
                "damage|mon:Bulbasaur,player-1,1|health:68/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:65/105",
                "damage|mon:Bulbasaur,player-1,1|health:62/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:58/105",
                "damage|mon:Bulbasaur,player-1,1|health:56/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:51/105",
                "damage|mon:Bulbasaur,player-1,1|health:49/100",
                "hitcount|hits:5",
                "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1|notarget",
                "miss|mon:Bulbasaur,player-2,1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:43/105",
                "damage|mon:Bulbasaur,player-1,1|health:41/100",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:36/105",
                "damage|mon:Bulbasaur,player-1,1|health:35/100",
                "hitcount|hits:2",
                "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:78/105",
                "damage|mon:Bulbasaur,player-2,1|health:75/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:71/105",
                "damage|mon:Bulbasaur,player-2,1|health:68/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:64/105",
                "damage|mon:Bulbasaur,player-2,1|health:61/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:58/105",
                "damage|mon:Bulbasaur,player-2,1|health:56/100",
                "hitcount|hits:4",
                "residual",
                "turn|turn:4"
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
                "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:98/105",
                "damage|mon:Bulbasaur,player-2,1|health:94/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:92/105",
                "damage|mon:Bulbasaur,player-2,1|health:88/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:86/105",
                "damage|mon:Bulbasaur,player-2,1|health:82/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:79/105",
                "damage|mon:Bulbasaur,player-2,1|health:76/100",
                "hitcount|hits:4",
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
                "damage|mon:Bulbasaur,player-2,1|health:72/105",
                "damage|mon:Bulbasaur,player-2,1|health:69/100",
                "crit|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:61/105",
                "damage|mon:Bulbasaur,player-2,1|health:59/100",
                "crit|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:50/105",
                "damage|mon:Bulbasaur,player-2,1|health:48/100",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:43/105",
                "damage|mon:Bulbasaur,player-2,1|health:41/100",
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
                "move|mon:Bulbasaur,player-2,1|name:Double Kick|target:Bulbasaur,player-1,1",
                "resisted|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:99/105",
                "damage|mon:Bulbasaur,player-1,1|health:95/100",
                "resisted|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:92/105",
                "damage|mon:Bulbasaur,player-1,1|health:88/100",
                "hitcount|hits:2",
                "move|mon:Bulbasaur,player-1,1|name:Double Kick|target:Bulbasaur,player-2,1",
                "resisted|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:99/105",
                "damage|mon:Bulbasaur,player-2,1|health:95/100",
                "resisted|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:92/105",
                "damage|mon:Bulbasaur,player-2,1|health:88/100",
                "hitcount|hits:2",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Double Kick|target:Bulbasaur,player-1,1",
                "resisted|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:86/105",
                "damage|mon:Bulbasaur,player-1,1|health:82/100",
                "resisted|mon:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:80/105",
                "damage|mon:Bulbasaur,player-1,1|health:77/100",
                "hitcount|hits:2",
                "move|mon:Bulbasaur,player-1,1|name:Double Kick|target:Bulbasaur,player-2,1",
                "resisted|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:86/105",
                "damage|mon:Bulbasaur,player-2,1|health:82/100",
                "resisted|mon:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:80/105",
                "damage|mon:Bulbasaur,player-2,1|health:77/100",
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
                "damage|mon:Bulbasaur,player-2,1|health:13/105",
                "damage|mon:Bulbasaur,player-2,1|health:13/100",
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

    #[test]
    fn second_hit_can_apply_secondary_effect() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 101217792730310).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

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
                "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:50|gender:M",
                "move|mon:Bulbasaur,player-1,1|name:Twineedle|target:Charmander,player-2,1",
                "resisted|mon:Charmander,player-2,1",
                "split|side:1",
                "damage|mon:Charmander,player-2,1|health:93/99",
                "damage|mon:Charmander,player-2,1|health:94/100",
                "resisted|mon:Charmander,player-2,1",
                "split|side:1",
                "damage|mon:Charmander,player-2,1|health:87/99",
                "damage|mon:Charmander,player-2,1|health:88/100",
                "status|mon:Charmander,player-2,1|status:Poison",
                "hitcount|hits:2",
                "split|side:1",
                "damage|mon:Charmander,player-2,1|from:status:Poison|health:75/99",
                "damage|mon:Charmander,player-2,1|from:status:Poison|health:76/100",
                "residual",
                "turn|turn:2"
            ]"#,
        ).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
