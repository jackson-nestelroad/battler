#[cfg(test)]
mod move_usage_tests {
    use assert_matches::assert_matches;
    use battler::{
        battle::{
            Battle,
            BattleType,
            PublicCoreBattle,
            Request,
        },
        common::{
            Error,
            Id,
            WrapResultError,
        },
        dex::DataStore,
        moves::MoveData,
        teams::TeamData,
    };
    use battler_test_utils::{
        TestBattleBuilder,
        TestDataStore,
    };

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": ["Test Move 1", "Test Move 2"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 100
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_actual_health(true)
            .with_pass_allowed(true)
            .with_team_validation(false)
            .add_player_to_side_1("test-player", "Player 1")
            .add_player_to_side_2("foe", "Player 2")
            .with_team("test-player", team()?)
            .with_team("foe", team()?)
            .build(data)
    }

    fn test_move(name: &str) -> Result<MoveData, Error> {
        let mut move_data: MoveData = serde_json::from_str(
            r#"{
                "name": "",
                "category": "Physical",
                "primary_type": "Normal",
                "base_power": 1,
                "accuracy": "exempt",
                "pp": 5,
                "target": "Normal",
                "flags": []
            }"#,
        )
        .wrap_error()?;
        move_data.name = name.to_owned();
        Ok(move_data)
    }

    fn add_test_moves(data: &mut TestDataStore) -> Result<(), Error> {
        data.add_fake_move(Id::from("Test Move 1"), test_move("Test Move 1")?);
        data.add_fake_move(Id::from("Test Move 2"), test_move("Test Move 2")?);
        Ok(())
    }

    #[test]
    fn using_move_reduces_pp() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        add_test_moves(&mut data).unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        let request = battle.request_for_player("test-player");
        assert_matches!(request, Some(Request::Turn(request)) => {
            assert_eq!(request.active[0].moves[0].pp, 5);
            assert_eq!(request.active[0].moves[1].pp, 5);
        });

        assert_eq!(battle.set_player_choice("foe", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("test-player", "move 0"), Ok(()));

        let request = battle.request_for_player("test-player");
        assert_matches!(request, Some(Request::Turn(request)) => {
            assert_eq!(request.active[0].moves[0].pp, 4);
            assert_eq!(request.active[0].moves[1].pp, 5);
        });

        assert_eq!(battle.set_player_choice("foe", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("test-player", "move 0"), Ok(()));

        let request = battle.request_for_player("test-player");
        assert_matches!(request, Some(Request::Turn(request)) => {
            assert_eq!(request.active[0].moves[0].pp, 3);
            assert_eq!(request.active[0].moves[1].pp, 5);
        });

        assert_eq!(battle.set_player_choice("foe", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("test-player", "move 1"), Ok(()));

        let request = battle.request_for_player("test-player");
        assert_matches!(request, Some(Request::Turn(request)) => {
            assert_eq!(request.active[0].moves[0].pp, 3);
            assert_eq!(request.active[0].moves[1].pp, 4);
        });
    }
}
