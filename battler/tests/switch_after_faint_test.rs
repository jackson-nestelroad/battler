#[cfg(test)]
mod switch_after_faint_test {
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
            WrapResultError,
        },
        dex::{
            DataStore,
            LocalDataStore,
        },
        teams::TeamData,
    };
    use battler_test_utils::{
        assert_error_message,
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "Overgrow",
                        "moves": ["Tackle", "Air Cutter"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Charmander",
                        "species": "Charmander",
                        "ability": "Blaze",
                        "moves": ["Scratch"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 5
                    },
                    {
                        "name": "Squirtle",
                        "species": "Squirtle",
                        "ability": "Torrent",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 5
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn make_battle_builder() -> TestBattleBuilder {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .with_seed(0)
            .with_team_validation(false)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
    }

    fn make_battle(data: &dyn DataStore, auto_continue: bool) -> Result<PublicCoreBattle, Error> {
        make_battle_builder()
            .with_auto_continue(auto_continue)
            .with_team("player-1", team()?)
            .with_team("player-2", team()?)
            .build(data)
    }

    #[test]
    fn must_switch_after_faint() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, true).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "move 0,2;move 0,1"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 0,1;move 0,1"),
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
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Charmander|health:100/100|species:Charmander|level:5|gender:F",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Charmander|health:100/100|species:Charmander|level:5|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:86/105",
                "damage|mon:Bulbasaur,player-1,1|health:82/100",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Charmander,player-2,2",
                "split|side:1",
                "damage|mon:Charmander,player-2,2|health:0",
                "damage|mon:Charmander,player-2,2|health:0",
                "faint|mon:Charmander,player-2,2",
                "move|mon:Charmander,player-1,2|name:Scratch|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:104/105",
                "damage|mon:Bulbasaur,player-2,1|health:99/100",
                "residual"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        assert_matches!(battle.request_for_player("player-1"), None);
        assert_matches!(battle.request_for_player("player-2"), Some(Request::Switch(request)) => {
            assert_eq!(request.needs_switch, vec![1]);
        });

        assert_error_message(
            battle.set_player_choice("player-1", "switch 2"),
            "you cannot do anything: no action requested",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "move 0,2;move 0,1"),
            "cannot move: you cannot move out of turn",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "switch 0"),
            "cannot switch: you cannot switch to an active Mon",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "switch 1"),
            "cannot switch: you cannot switch to an active Mon",
        );
        assert_eq!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                ["time"],
                "switch|player:player-2|position:2|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:F",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        assert_matches!(battle.request_for_player("player-1"), Some(Request::Turn(request)) => {
            assert_eq!(request.active.len(), 2);
            assert_eq!(request.player.mons[request.active[0].team_position].name, "Bulbasaur");
            assert_eq!(request.player.mons[request.active[1].team_position].name, "Charmander");
        });
        assert_matches!(battle.request_for_player("player-2"), Some(Request::Turn(request)) => {
            assert_eq!(request.active.len(), 2);
            assert_eq!(request.player.mons[request.active[0].team_position].name, "Bulbasaur");
            assert_eq!(request.player.mons[request.active[1].team_position].name, "Squirtle");
        });

        assert_eq!(
            battle.set_player_choice("player-1", "move 0,2;move 0,1"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 0,1;move 0,1"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                ["time"],
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Squirtle,player-2,2",
                "split|side:1",
                "damage|mon:Squirtle,player-2,2|health:0",
                "damage|mon:Squirtle,player-2,2|health:0",
                "faint|mon:Squirtle,player-2,2",
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:68/105",
                "damage|mon:Bulbasaur,player-1,1|health:65/100",
                "move|mon:Charmander,player-1,2|name:Scratch|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:103/105",
                "damage|mon:Bulbasaur,player-2,1|health:99/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        assert_matches!(battle.request_for_player("player-1"), Some(Request::Turn(request)) => {
            assert_eq!(request.active.len(), 2);
            assert_eq!(request.player.mons[request.active[0].team_position].name, "Bulbasaur");
            assert_eq!(request.player.mons[request.active[1].team_position].name, "Charmander");
        });
        assert_matches!(battle.request_for_player("player-2"), Some(Request::Turn(request)) => {
            assert_eq!(request.active.len(), 1);
            assert_eq!(request.player.mons[request.active[0].team_position].name, "Bulbasaur");
        });
    }

    #[test]
    fn must_switch_one_after_two_faint() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, false).unwrap();
        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.continue_battle(), Ok(()));

        assert_eq!(
            battle.set_player_choice("player-1", "move 1;move 0,1"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "switch 2;move 0,1"),
            Ok(())
        );
        assert_eq!(battle.continue_battle(), Ok(()));

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
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-1|position:2|name:Charmander|health:100/100|species:Charmander|level:5|gender:F",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Charmander|health:100/100|species:Charmander|level:5|gender:F",
                "turn|turn:1",
                ["time"],
                "switch|player:player-2|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:F",
                "move|mon:Bulbasaur,player-1,1|name:Air Cutter|spread:Squirtle,player-2,1;Charmander,player-2,2",
                "split|side:1",
                "damage|mon:Squirtle,player-2,1|health:0",
                "damage|mon:Squirtle,player-2,1|health:0",
                "split|side:1",
                "damage|mon:Charmander,player-2,2|health:0",
                "damage|mon:Charmander,player-2,2|health:0",
                "faint|mon:Squirtle,player-2,1",
                "faint|mon:Charmander,player-2,2",
                "move|mon:Charmander,player-1,2|name:Scratch|notarget",
                "fail|mon:Charmander,player-1,2",
                "residual"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        assert_matches!(battle.request_for_player("player-1"), None);
        assert_matches!(battle.request_for_player("player-2"), Some(Request::Switch(request)) => {
            assert_eq!(request.needs_switch, vec![0, 1]);
        });

        assert_error_message(
            battle.set_player_choice("player-1", "switch 2"),
            "you cannot do anything: no action requested",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "move 0,2;move 0,1"),
            "cannot move: you cannot move out of turn",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "switch 1"),
            "cannot switch: you cannot switch to an active Mon",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "switch 0;switch 2"),
            "cannot switch: you cannot switch to an active Mon",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "switch 0;switch 0"),
            "cannot switch: the Mon in slot 0 can only switch in once",
        );
        assert_error_message(
            battle.set_player_choice("player-2", "switch 0"),
            "incomplete choice: missing actions for Mons",
        );

        // We have a choice as to where the single Mon can be switched into.
        assert_eq!(
            battle.set_player_choice("player-2", "switch 0;pass"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "pass;switch 0"),
            Ok(())
        );
        assert_eq!(battle.continue_battle(), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                ["time"],
                "switch|player:player-2|position:2|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:F",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        assert_matches!(battle.request_for_player("player-1"), Some(Request::Turn(request)) => {
            assert_eq!(request.active.len(), 2);
            assert_eq!(request.player.mons[request.active[0].team_position].name, "Bulbasaur");
            assert_eq!(request.player.mons[request.active[1].team_position].name, "Charmander");
        });
        assert_matches!(battle.request_for_player("player-2"), Some(Request::Turn(request)) => {
            assert_eq!(request.active.len(), 1);
            assert_eq!(request.player.mons[request.active[0].team_position].name, "Bulbasaur");
        });
    }
}