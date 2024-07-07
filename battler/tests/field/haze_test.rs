#[cfg(test)]
mod haze_test {
    use battler::{
        battle::{
            BattleType,
            CoreBattleEngineRandomizeBaseDamage,
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

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Weezing",
                        "species": "Weezing",
                        "ability": "No Ability",
                        "moves": [
                            "Haze",
                            "Double Team",
                            "Growth",
                            "Body Slam"
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
            .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn haze_clears_all_stat_changes() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 98371927, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

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
                "switch|player:player-1|position:1|name:Weezing|health:100/100|species:Weezing|level:50|gender:M",
                "switch|player:player-2|position:1|name:Weezing|health:100/100|species:Weezing|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Weezing,player-2,1|name:Double Team|target:Weezing,player-2,1",
                "boost|mon:Weezing,player-2,1|stat:eva|by:1",
                "move|mon:Weezing,player-1,1|name:Growth|target:Weezing,player-1,1",
                "boost|mon:Weezing,player-1,1|stat:atk|by:1",
                "boost|mon:Weezing,player-1,1|stat:spa|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Weezing,player-2,1|name:Double Team|target:Weezing,player-2,1",
                "boost|mon:Weezing,player-2,1|stat:eva|by:1",
                "move|mon:Weezing,player-1,1|name:Growth|target:Weezing,player-1,1",
                "boost|mon:Weezing,player-1,1|stat:atk|by:1",
                "boost|mon:Weezing,player-1,1|stat:spa|by:1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Weezing,player-1,1|name:Haze",
                "clearallboosts",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Weezing,player-2,1|name:Body Slam|target:Weezing,player-1,1",
                "split|side:0",
                "damage|mon:Weezing,player-1,1|health:95/125",
                "damage|mon:Weezing,player-1,1|health:76/100",
                "move|mon:Weezing,player-1,1|name:Body Slam|target:Weezing,player-2,1",
                "split|side:1",
                "damage|mon:Weezing,player-2,1|health:95/125",
                "damage|mon:Weezing,player-2,1|health:76/100",
                "status|mon:Weezing,player-2,1|status:Paralysis",
                "residual",
                "turn|turn:5"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
