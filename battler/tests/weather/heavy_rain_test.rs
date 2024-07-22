#[cfg(test)]
mod rain_test {
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

    fn blastoise() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Blastoise",
                        "species": "Blastoise",
                        "ability": "No Ability",
                        "moves": [
                            "Rain Dance",
                            "Water Gun",
                            "Thunder",
                            "Embargo"
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

    fn charizard() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "No Ability",
                        "moves": [
                            "Flamethrower",
                            "Double Team"
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

    fn kyogre() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Kyogre",
                        "species": "Kyogre",
                        "ability": "Primordial Sea",
                        "moves": [
                            "Water Gun"
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

    fn kyogre_charizard() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "No Ability",
                        "moves": [
                            "Flamethrower"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Kyogre",
                        "species": "Kyogre",
                        "ability": "Primordial Sea",
                        "moves": [
                            "Water Gun"
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
            .with_controlled_rng(true)
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
    fn primordial_sea_starts_heavy_rain_on_switch_in() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, kyogre().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Kyogre|health:100/100|species:Kyogre|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "weather|weather:Heavy Rain|from:ability:Primordial Sea|of:Kyogre,player-1,1",
                "turn|turn:1",
                ["time"],
                "weather|weather:Heavy Rain|residual",
                "residual",
                "turn|turn:2",
                ["time"],
                "weather|weather:Heavy Rain|residual",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn primordial_sea_dissipates_fire_type_moves() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, kyogre().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Kyogre|health:100/100|species:Kyogre|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "weather|weather:Heavy Rain|from:ability:Primordial Sea|of:Kyogre,player-1,1",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Flamethrower|noanim",
                "fail|mon:Charizard,player-2,1|from:weather:Heavy Rain",
                "weather|weather:Heavy Rain|residual",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn normal_rain_cannot_override_primordial_sea() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, kyogre().unwrap(), blastoise().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Kyogre|health:100/100|species:Kyogre|level:50|gender:M",
                "switch|player:player-2|position:1|name:Blastoise|health:100/100|species:Blastoise|level:50|gender:M",
                "weather|weather:Heavy Rain|from:ability:Primordial Sea|of:Kyogre,player-1,1",
                "turn|turn:1",
                ["time"],
                "move|mon:Blastoise,player-2,1|name:Rain Dance|noanim",
                "fail|mon:Blastoise,player-2,1",
                "weather|weather:Heavy Rain|residual",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn primordial_sea_stops_when_last_mon_with_ability_switches_out() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            0,
            kyogre_charizard().unwrap(),
            kyogre_charizard().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 0"), Ok(()));

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
                "switch|player:player-1|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "switch|player:player-2|position:1|name:Kyogre|health:100/100|species:Kyogre|level:50|gender:M",
                "switch|player:player-1|position:1|name:Kyogre|health:100/100|species:Kyogre|level:50|gender:M",
                "weather|weather:Heavy Rain|from:ability:Primordial Sea|of:Kyogre,player-1,1",
                "weather|weather:Heavy Rain|residual",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-1|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "weather|weather:Heavy Rain|residual",
                "residual",
                "turn|turn:3",
                ["time"],
                "weather|weather:Clear",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "move|mon:Charizard,player-1,1|name:Flamethrower|target:Charizard,player-2,1",
                "resisted|mon:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:101/138",
                "damage|mon:Charizard,player-2,1|health:74/100",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
