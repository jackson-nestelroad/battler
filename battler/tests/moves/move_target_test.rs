#[cfg(test)]
mod move_target_tests {
    use battler::{
        battle::{
            Battle,
            BattleEngineSpeedSortTieResolution,
            BattleType,
            PublicCoreBattle,
        },
        common::{
            Error,
            Id,
            WrapResultError,
        },
        dex::DataStore,
        moves::{
            MoveData,
            MoveTarget,
        },
        teams::TeamData,
    };
    use battler_test_utils::{
        assert_new_logs_eq,
        LogMatch,
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
                        "moves": ["Test Move"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": ["Test Move"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn test_move(target: MoveTarget) -> Result<MoveData, Error> {
        let mut move_data: MoveData = serde_json::from_str(
            r#"{
                "name": "Test Move",
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
        move_data.target = target;
        Ok(move_data)
    }

    fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_seed(2)
            .with_battle_type(BattleType::Doubles)
            .with_pass_allowed(true)
            .with_team_validation(false)
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("test-player", "Test Player")
            .add_player_to_side_2("foe", "Foe")
            .with_team("test-player", team()?)
            .with_team("foe", team()?)
            .build(data)
    }

    #[test]
    fn can_hit_adjacent_ally() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(
            Id::from("Test Move"),
            test_move(MoveTarget::AdjacentAlly).unwrap(),
        );
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0,-2;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|target:Venusaur,test-player,2",
                "split|side:0",
                "damage|mon:Venusaur,test-player,2|health:139/140",
                "damage|mon:Venusaur,test-player,2|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_adjacent_ally_or_user() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(
            Id::from("Test Move"),
            test_move(MoveTarget::AdjacentAllyOrUser).unwrap(),
        );
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0,-1;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|target:Venusaur,test-player,1",
                "split|side:0",
                "damage|mon:Venusaur,test-player,1|health:139/140",
                "damage|mon:Venusaur,test-player,1|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_adjacent_foe() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(
            Id::from("Test Move"),
            test_move(MoveTarget::AdjacentFoe).unwrap(),
        );
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0,2;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|target:Venusaur,foe,2",
                "split|side:1",
                "damage|mon:Venusaur,foe,2|health:139/140",
                "damage|mon:Venusaur,foe,2|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_all_adjacent() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(
            Id::from("Test Move"),
            test_move(MoveTarget::AllAdjacent).unwrap(),
        );
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|spread:Venusaur,test-player,2;Venusaur,foe,1;Venusaur,foe,2",
                "split|side:0",
                "damage|mon:Venusaur,test-player,2|health:139/140",
                "damage|mon:Venusaur,test-player,2|health:99/100",
                "split|side:1",
                "damage|mon:Venusaur,foe,1|health:139/140",
                "damage|mon:Venusaur,foe,1|health:99/100",
                "split|side:1",
                "damage|mon:Venusaur,foe,2|health:139/140",
                "damage|mon:Venusaur,foe,2|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_all_adjacent_foes() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(
            Id::from("Test Move"),
            test_move(MoveTarget::AllAdjacentFoes).unwrap(),
        );
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|spread:Venusaur,foe,1;Venusaur,foe,2",
                "split|side:1",
                "damage|mon:Venusaur,foe,1|health:139/140",
                "damage|mon:Venusaur,foe,1|health:99/100",
                "split|side:1",
                "damage|mon:Venusaur,foe,2|health:139/140",
                "damage|mon:Venusaur,foe,2|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_allies() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(
            Id::from("Test Move"),
            test_move(MoveTarget::Allies).unwrap(),
        );
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|spread:Venusaur,test-player,2;Venusaur,test-player,1",
                "split|side:0",
                "damage|mon:Venusaur,test-player,2|health:139/140",
                "damage|mon:Venusaur,test-player,2|health:99/100",
                "split|side:0",
                "damage|mon:Venusaur,test-player,1|health:139/140",
                "damage|mon:Venusaur,test-player,1|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_any() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(Id::from("Test Move"), test_move(MoveTarget::Any).unwrap());
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0,1;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|target:Venusaur,foe,1",
                "split|side:1",
                "damage|mon:Venusaur,foe,1|health:139/140",
                "damage|mon:Venusaur,foe,1|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_normal() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(
            Id::from("Test Move"),
            test_move(MoveTarget::Normal).unwrap(),
        );
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0,2;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|target:Venusaur,foe,2",
                "split|side:1",
                "damage|mon:Venusaur,foe,2|health:139/140",
                "damage|mon:Venusaur,foe,2|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_random_normal() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(
            Id::from("Test Move"),
            test_move(MoveTarget::RandomNormal).unwrap(),
        );
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|target:Venusaur,foe,1",
                "split|side:1",
                "damage|mon:Venusaur,foe,1|health:138/140",
                "damage|mon:Venusaur,foe,1|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn can_hit_user() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        data.add_fake_move(Id::from("Test Move"), test_move(MoveTarget::User).unwrap());
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("foe", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("test-player", "move 0;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:test-player|name:Test Player|side:0|position:0",
                "player|id:foe|name:Foe|side:1|position:0",
                ["time"],
                "teamsize|player:test-player|size:2",
                "teamsize|player:foe|size:2",
                "start",
                "switch|player:test-player|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:test-player|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:foe|position:2|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,test-player,1|name:Test Move|target:Venusaur,test-player,1",
                "split|side:0",
                "damage|mon:Venusaur,test-player,1|health:139/140",
                "damage|mon:Venusaur,test-player,1|health:99/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
