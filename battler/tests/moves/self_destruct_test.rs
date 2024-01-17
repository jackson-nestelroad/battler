#[cfg(test)]
mod self_destruct_tests {
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
        assert_turn_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn test_team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": ["Self-Destruct"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn foe_team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_seed(0)
            .with_battle_type(BattleType::Singles)
            .with_team_validation(false)
            .add_player_to_side_1("test-player", "Test Player")
            .add_player_to_side_2("foe", "Foe")
            .with_team("test-player", test_team()?)
            .with_team("foe", foe_team()?)
            .build(data)
    }

    #[test]
    fn self_destruct_loses() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("test-player", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("foe", "move 0"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Venusaur,test-player,1|name:Self-Destruct",
                "split|side:1",
                "damage|mon:Venusaur,foe,1|health:0",
                "damage|mon:Venusaur,foe,1|health:0",
                "faint|mon:Venusaur,test-player,1",
                "faint|mon:Venusaur,foe,1",
                "win|side:1"
            ]"#,
        )
        .unwrap();
        assert_turn_logs_eq(&mut battle, 1, &expected_logs);
    }
}
