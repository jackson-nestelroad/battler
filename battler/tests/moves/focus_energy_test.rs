#[cfg(test)]
mod focus_energy_test {
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
                        "name": "Nidoking",
                        "species": "Nidoking",
                        "ability": "No Ability",
                        "moves": [
                            "Focus Energy",
                            "Spike Cannon"
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
        battle_type: BattleType,
        seed: u64,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(battle_type)
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
    fn focus_energy_increases_crit_ratio() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            BattleType::Singles,
            0,
            team().unwrap(),
            team().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
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
                "switch|player:player-1|position:1|name:Nidoking|health:100/100|species:Nidoking|level:50|gender:M",
                "switch|player:player-2|position:1|name:Nidoking|health:100/100|species:Nidoking|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Nidoking,player-1,1|name:Focus Energy|target:Nidoking,player-1,1",
                "start|mon:Nidoking,player-1,1|move:Focus Energy",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
                "crit|mon:Nidoking,player-2,1",
                "split|side:1",
                "damage|mon:Nidoking,player-2,1|health:122/141",
                "damage|mon:Nidoking,player-2,1|health:87/100",
                "crit|mon:Nidoking,player-2,1",
                "split|side:1",
                "damage|mon:Nidoking,player-2,1|health:105/141",
                "damage|mon:Nidoking,player-2,1|health:75/100",
                "split|side:1",
                "damage|mon:Nidoking,player-2,1|health:94/141",
                "damage|mon:Nidoking,player-2,1|health:67/100",
                "hitcount|hits:3",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
                "crit|mon:Nidoking,player-2,1",
                "split|side:1",
                "damage|mon:Nidoking,player-2,1|health:77/141",
                "damage|mon:Nidoking,player-2,1|health:55/100",
                "split|side:1",
                "damage|mon:Nidoking,player-2,1|health:65/141",
                "damage|mon:Nidoking,player-2,1|health:47/100",
                "split|side:1",
                "damage|mon:Nidoking,player-2,1|health:53/141",
                "damage|mon:Nidoking,player-2,1|health:38/100",
                "hitcount|hits:3",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
