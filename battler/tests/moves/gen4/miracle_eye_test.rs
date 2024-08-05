#[cfg(test)]
mod miracle_eye_test {
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

    fn machamp() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Xatu",
                        "species": "Xatu",
                        "ability": "No Ability",
                        "moves": [
                            "Miracle Eye",
                            "Psychic"
                        ],
                        "nature": "Hardy",
                        "level": 50
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn gengar() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Umbreon",
                        "species": "Umbreon",
                        "ability": "No Ability",
                        "moves": [
                            "Double Team"
                        ],
                        "nature": "Hardy",
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
    fn miracle_eye_ignores_evasion_and_removes_dark_type_immunity_of_psychic_type() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, machamp().unwrap(), gengar().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Xatu,player-1,1|name:Miracle Eye|target:Umbreon,player-2,1",
                "start|mon:Umbreon,player-2,1|move:Miracle Eye",
                "move|mon:Umbreon,player-2,1|name:Double Team|target:Umbreon,player-2,1",
                "boost|mon:Umbreon,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Xatu,player-1,1|name:Psychic|target:Umbreon,player-2,1",
                "split|side:1",
                "damage|mon:Umbreon,player-2,1|health:110/155",
                "damage|mon:Umbreon,player-2,1|health:71/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
