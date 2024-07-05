#[cfg(test)]
mod leech_seed_test {
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
                        "name": "Eevee",
                        "species": "Eevee",
                        "ability": "No Ability",
                        "moves": [
                            "Leech Seed",
                            "Brick Break"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Exeggcute",
                        "species": "Exeggcute",
                        "ability": "No Ability",
                        "moves": [
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
            .with_battle_type(BattleType::Singles)
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
    fn leech_seed_leeches_user_until_switch_out() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 9284091283, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        // Switch out and show that target is still seeded, healing the switched-in Mon.
        assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        // Effect should end.
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Eevee|health:100/100|species:Eevee|level:50|gender:M",
                "switch|player:player-2|position:1|name:Eevee|health:100/100|species:Eevee|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Eevee,player-2,1|name:Brick Break|target:Eevee,player-1,1",
                "supereffective|mon:Eevee,player-1,1",
                "split|side:0",
                "damage|mon:Eevee,player-1,1|health:43/115",
                "damage|mon:Eevee,player-1,1|health:38/100",
                "move|mon:Eevee,player-1,1|name:Leech Seed|target:Eevee,player-2,1",
                "start|mon:Eevee,player-2,1|move:Leech Seed",
                "split|side:1",
                "damage|mon:Eevee,player-2,1|from:move:Leech Seed|health:101/115",
                "damage|mon:Eevee,player-2,1|from:move:Leech Seed|health:88/100",
                "split|side:0",
                "heal|mon:Eevee,player-1,1|from:move:Leech Seed|of:Eevee,player-2,1|health:57/115",
                "heal|mon:Eevee,player-1,1|from:move:Leech Seed|of:Eevee,player-2,1|health:50/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "split|side:1",
                "damage|mon:Eevee,player-2,1|from:move:Leech Seed|health:87/115",
                "damage|mon:Eevee,player-2,1|from:move:Leech Seed|health:76/100",
                "split|side:0",
                "heal|mon:Eevee,player-1,1|from:move:Leech Seed|of:Eevee,player-2,1|health:71/115",
                "heal|mon:Eevee,player-1,1|from:move:Leech Seed|of:Eevee,player-2,1|health:62/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "split|side:1",
                "damage|mon:Eevee,player-2,1|from:move:Leech Seed|health:73/115",
                "damage|mon:Eevee,player-2,1|from:move:Leech Seed|health:64/100",
                "split|side:0",
                "heal|mon:Eevee,player-1,1|from:move:Leech Seed|of:Eevee,player-2,1|health:85/115",
                "heal|mon:Eevee,player-1,1|from:move:Leech Seed|of:Eevee,player-2,1|health:74/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "switch|player:player-1|position:1|name:Exeggcute|health:100/100|species:Exeggcute|level:50|gender:M",
                "move|mon:Eevee,player-2,1|name:Brick Break|target:Exeggcute,player-1,1",
                "resisted|mon:Exeggcute,player-1,1",
                "split|side:0",
                "damage|mon:Exeggcute,player-1,1|health:108/120",
                "damage|mon:Exeggcute,player-1,1|health:90/100",
                "split|side:1",
                "damage|mon:Eevee,player-2,1|from:move:Leech Seed|health:59/115",
                "damage|mon:Eevee,player-2,1|from:move:Leech Seed|health:52/100",
                "split|side:0",
                "heal|mon:Exeggcute,player-1,1|from:move:Leech Seed|of:Eevee,player-2,1|health:120/120",
                "heal|mon:Exeggcute,player-1,1|from:move:Leech Seed|of:Eevee,player-2,1|health:100/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "switch|player:player-2|position:1|name:Exeggcute|health:100/100|species:Exeggcute|level:50|gender:M",
                "residual",
                "turn|turn:6"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn grass_types_resist_leech_seed() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
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
                "switch|player:player-1|position:1|name:Eevee|health:100/100|species:Eevee|level:50|gender:M",
                "switch|player:player-2|position:1|name:Eevee|health:100/100|species:Eevee|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "switch|player:player-2|position:1|name:Exeggcute|health:100/100|species:Exeggcute|level:50|gender:M",
                "move|mon:Eevee,player-1,1|name:Leech Seed|noanim",
                "immune|mon:Exeggcute,player-2,1",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
