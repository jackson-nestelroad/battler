#[cfg(test)]
mod dream_eater_test {
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
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn gengar() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Gengar",
                        "species": "Gengar",
                        "ability": "No Ability",
                        "moves": [
                            "Dream Eater",
                            "Sleep Powder",
                            "Flamethrower"
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
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn dream_eater_drains_damage_of_sleeping_foe() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, gengar().unwrap(), gengar().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Gengar|health:100/100|species:Gengar|level:50|gender:M",
                "switch|player:player-2|position:1|name:Gengar|health:100/100|species:Gengar|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Gengar,player-2,1|name:Flamethrower|target:Gengar,player-1,1",
                "split|side:0",
                "damage|mon:Gengar,player-1,1|health:55/120",
                "damage|mon:Gengar,player-1,1|health:46/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Gengar,player-1,1|name:Dream Eater|noanim",
                "immune|mon:Gengar,player-2,1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Gengar,player-1,1|name:Sleep Powder|target:Gengar,player-2,1",
                "status|mon:Gengar,player-2,1|status:Sleep|from:move:Sleep Powder",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Gengar,player-1,1|name:Dream Eater|target:Gengar,player-2,1",
                "supereffective|mon:Gengar,player-2,1",
                "split|side:1",
                "damage|mon:Gengar,player-2,1|health:0",
                "damage|mon:Gengar,player-2,1|health:0",
                "split|side:0",
                "heal|mon:Gengar,player-1,1|from:Drain|of:Gengar,player-2,1|health:115/120",
                "heal|mon:Gengar,player-1,1|from:Drain|of:Gengar,player-2,1|health:96/100",
                "faint|mon:Gengar,player-2,1",
                "win|side:0"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
