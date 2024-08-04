#[cfg(test)]
mod attract_test {
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

    fn male_wobbuffet() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Wobbuffet",
                        "species": "Wobbuffet",
                        "ability": "No Ability",
                        "moves": [
                            "Attract",
                            "Tackle"
                        ],
                        "nature": "Hardy",
                        "level": 50,
                        "gender": "Male"
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn female_wobbuffet() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Wobbuffet",
                        "species": "Wobbuffet",
                        "ability": "No Ability",
                        "moves": [
                            "Attract",
                            "Tackle"
                        ],
                        "nature": "Hardy",
                        "level": 50,
                        "gender": "Female"
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn female_wobbuffet_with_destiny_knot() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Wobbuffet",
                        "species": "Wobbuffet",
                        "ability": "No Ability",
                        "moves": [
                            "Attract",
                            "Tackle"
                        ],
                        "nature": "Hardy",
                        "level": 50,
                        "gender": "Female",
                        "item": "Destiny Knot"
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn unown() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Unown",
                        "species": "Unown",
                        "ability": "No Ability",
                        "moves": [
                            "Attract",
                            "Tackle"
                        ],
                        "nature": "Hardy",
                        "level": 50,
                        "gender": "Unknown"
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
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Reverse)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn attract_causes_infatuation() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            0,
            male_wobbuffet().unwrap(),
            female_wobbuffet().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Wobbuffet,player-1,1|name:Attract|target:Wobbuffet,player-2,1",
                "start|mon:Wobbuffet,player-2,1|move:Attract",
                "residual",
                "turn|turn:2",
                ["time"],
                "activate|move:Attract|of:Wobbuffet,player-1,1",
                "cant|mon:Wobbuffet,player-2,1|reason:Attract",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn destiny_knot_causes_mutual_attraction() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            0,
            male_wobbuffet().unwrap(),
            female_wobbuffet_with_destiny_knot().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Wobbuffet,player-1,1|name:Attract|target:Wobbuffet,player-2,1",
                "start|mon:Wobbuffet,player-1,1|move:Attract|from:item:Destiny Knot|of:Wobbuffet,player-2,1",
                "start|mon:Wobbuffet,player-2,1|move:Attract",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn attract_fails_for_equal_genders() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            0,
            male_wobbuffet().unwrap(),
            male_wobbuffet().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Wobbuffet,player-1,1|name:Attract|noanim",
                "fail|mon:Wobbuffet,player-1,1",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn attract_fails_for_unknown_gender() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, 0, unown().unwrap(), male_wobbuffet().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Unown,player-1,1|name:Attract|noanim",
                "fail|mon:Unown,player-1,1",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
