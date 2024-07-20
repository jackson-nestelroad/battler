#[cfg(test)]
mod action_priority_tests {
    use battler::{
        battle::{
            BattleType,
            PublicCoreBattle,
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
                        "name": "Fast",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": [
                            "High Priority",
                            "Normal Priority",
                            "Low Priority"
                        ],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50,
                        "ivs": {
                            "spe": 31
                        }
                    },
                    {
                        "name": "Slow",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": [
                            "High Priority",
                            "Normal Priority",
                            "Low Priority"
                        ],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50,
                        "ivs": {
                            "spe": 0
                        }
                    },
                    {
                        "name": "Extra",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": [
                            "High Priority",
                            "Normal Priority",
                            "Low Priority"
                        ],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50,
                        "ivs": {
                            "spe": 16
                        }
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn test_move(name: &str, priority: i8) -> Result<MoveData, Error> {
        let mut move_data: MoveData = serde_json::from_str(
            r#"{
                "name": "",
                "category": "Status",
                "primary_type": "Normal",
                "base_power": 0,
                "accuracy": "exempt",
                "pp": 5,
                "target": "Normal",
                "flags": []
            }"#,
        )
        .wrap_error()?;
        move_data.name = name.to_owned();
        move_data.priority = priority;
        Ok(move_data)
    }

    fn add_test_moves(data: &mut TestDataStore) -> Result<(), Error> {
        data.add_fake_move(Id::from("High Priority"), test_move("High Priority", 5)?);
        data.add_fake_move(
            Id::from("Normal Priority"),
            test_move("Normal Priority", 0)?,
        );
        data.add_fake_move(Id::from("Low Priority"), test_move("Low Priority", -5)?);
        Ok(())
    }

    fn test_battle_builder() -> Result<TestBattleBuilder, Error> {
        Ok(TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .with_pass_allowed(true)
            .with_team_validation(false)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team()?)
            .with_team("player-2", team()?))
    }

    fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        test_battle_builder()?.build(data)
    }

    #[test]
    fn switch_occurs_before_move() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        add_test_moves(&mut data).unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        // Switching out slow Mon occurs before high priority move.
        assert_eq!(
            battle.set_player_choice("player-1", "pass;switch 2"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 0,2;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "switch|player:player-1|position:2|name:Extra|health:100/100|species:Venusaur|level:50|gender:F",
                "move|mon:Fast,player-2,1|name:High Priority|target:Extra,player-1,2",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn switches_ordered_by_speed() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        add_test_moves(&mut data).unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "pass;switch 2"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "switch 2;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "switch|player:player-2|position:1|name:Extra|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Extra|health:100/100|species:Venusaur|level:50|gender:F",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn moves_ordered_by_speed() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        add_test_moves(&mut data).unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "pass;move 1,1"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 1,2;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Fast,player-2,1|name:Normal Priority|target:Slow,player-1,2",
                "move|mon:Slow,player-1,2|name:Normal Priority|target:Fast,player-2,1",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn moves_ordered_by_priority() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        add_test_moves(&mut data).unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "move 2,2;move 0,1"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 1,2;move 1,1"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Slow,player-1,2|name:High Priority|target:Fast,player-2,1",
                "move|mon:Fast,player-2,1|name:Normal Priority|target:Slow,player-1,2",
                "move|mon:Slow,player-2,2|name:Normal Priority|target:Fast,player-1,1",
                "move|mon:Fast,player-1,1|name:Low Priority|target:Slow,player-2,2",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    fn make_battle_with_seed(data: &dyn DataStore, seed: u64) -> Result<PublicCoreBattle, Error> {
        test_battle_builder()?.with_seed(seed).build(data)
    }

    #[test]
    fn speed_ties_broken_randomly() {
        let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
        add_test_moves(&mut data).unwrap();
        let mut battle = make_battle_with_seed(&data, 23567342414323).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "move 1,2;pass"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 1,2;pass"),
            Ok(())
        );

        assert_eq!(
            battle.set_player_choice("player-1", "move 1,2;pass"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 1,2;pass"),
            Ok(())
        );

        assert_eq!(
            battle.set_player_choice("player-1", "move 1,2;pass"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 1,2;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:1|name:Fast|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Slow|health:100/100|species:Venusaur|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Fast,player-2,1|name:Normal Priority|target:Slow,player-1,2",
                "move|mon:Fast,player-1,1|name:Normal Priority|target:Slow,player-2,2",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Fast,player-1,1|name:Normal Priority|target:Slow,player-2,2",
                "move|mon:Fast,player-2,1|name:Normal Priority|target:Slow,player-1,2",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Fast,player-2,1|name:Normal Priority|target:Slow,player-1,2",
                "move|mon:Fast,player-1,1|name:Normal Priority|target:Slow,player-2,2",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
