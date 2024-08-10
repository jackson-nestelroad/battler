#[cfg(test)]
mod thief_test {
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
        assert_logs_since_turn_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn crobat() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Crobat",
                        "species": "Crobat",
                        "ability": "No Ability",
                        "moves": [
                            "Thief"
                        ],
                        "nature": "Hardy",
                        "level": 50
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn crobat_with_goggles() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Crobat",
                        "species": "Crobat",
                        "ability": "No Ability",
                        "moves": [
                            "Thief"
                        ],
                        "nature": "Hardy",
                        "level": 50,
                        "item": "Safety Goggles"
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
            .with_weather(Some("sandstormweather".to_string()))
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn thief_steals_target_item() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, 0, crobat().unwrap(), crobat_with_goggles().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "weather|weather:Sandstorm|residual",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|from:weather:Sandstorm|health:136/145",
                "damage|mon:Crobat,player-1,1|from:weather:Sandstorm|health:94/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Crobat,player-1,1|name:Thief|target:Crobat,player-2,1",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|health:115/145",
                "damage|mon:Crobat,player-2,1|health:80/100",
                "itemend|mon:Crobat,player-2,1|item:Safety Goggles|silent|from:move:Thief|of:Crobat,player-1,1",
                "item|mon:Crobat,player-1,1|item:Safety Goggles|of:Crobat,player-2,1|from:move:Thief",
                "weather|weather:Sandstorm|residual",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|from:weather:Sandstorm|health:106/145",
                "damage|mon:Crobat,player-2,1|from:weather:Sandstorm|health:74/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
