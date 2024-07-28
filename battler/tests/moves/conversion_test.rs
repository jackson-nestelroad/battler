#[cfg(test)]
mod conversion_test {
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
        assert_logs_since_turn_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn normal_type_conversion() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Porygon",
                        "species": "Porygon",
                        "ability": "No Ability",
                        "moves": [
                            "Conversion",
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

    fn water_type_conversion() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Porygon",
                        "species": "Porygon",
                        "ability": "No Ability",
                        "moves": [
                            "Surf",
                            "Conversion",
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
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_controlled_rng(true)
            .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn conversion_sets_users_type() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            0,
            normal_type_conversion().unwrap(),
            water_type_conversion().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Porygon,player-2,1|name:Conversion|target:Porygon,player-2,1",
                "typechange|mon:Porygon,player-2,1|types:Water",
                "move|mon:Porygon,player-1,1|name:Conversion|noanim",
                "fail|mon:Porygon,player-1,1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Porygon,player-2,1|name:Water Gun|target:Porygon,player-1,1",
                "split|side:0",
                "damage|mon:Porygon,player-1,1|health:94/125",
                "damage|mon:Porygon,player-1,1|health:76/100",
                "move|mon:Porygon,player-1,1|name:Water Gun|target:Porygon,player-2,1",
                "resisted|mon:Porygon,player-2,1",
                "split|side:1",
                "damage|mon:Porygon,player-2,1|health:115/125",
                "damage|mon:Porygon,player-2,1|health:92/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
