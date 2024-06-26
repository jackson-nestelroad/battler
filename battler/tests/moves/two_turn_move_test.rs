#[cfg(test)]
mod two_turn_move_test {
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

    fn two_pidgeot() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pidgeot",
                        "species": "Pidgeot",
                        "ability": "No Ability",
                        "moves": [
                            "Razor Wind",
                            "Fly",
                            "Gust",
                            "Tackle"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Pidgeot",
                        "species": "Pidgeot",
                        "ability": "No Ability",
                        "moves": [
                            "Razor Wind",
                            "Fly",
                            "Gust",
                            "Tackle"
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
            .with_volatile_status_logs(true)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn razor_wind_uses_two_turns() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            10002323,
            two_pidgeot().unwrap(),
            two_pidgeot().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_lock_move_request = serde_json::from_str(
            r#"{
                "team_position": 0,
                "moves": [
                    {
                        "name": "Razor Wind",
                        "id": "razorwind",
                        "pp": 0,
                        "max_pp": 0,
                        "disabled": false
                    }
                ],
                "trapped": true
            }"#,
        )
        .unwrap();
        assert_eq!(
            battle
                .request_for_player("player-1")
                .map(|req| if let Request::Turn(req) = req {
                    req.active.get(0).cloned()
                } else {
                    None
                })
                .flatten(),
            Some(expected_lock_move_request)
        );

        assert_error_message(
            battle.set_player_choice("player-1", "move 1"),
            "cannot move: Pidgeot does not have a move in slot 1",
        );
        assert_error_message(
            battle.set_player_choice("player-1", "switch 1"),
            "cannot switch: Pidgeot is trapped",
        );

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Pidgeot|health:100/100|species:Pidgeot|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pidgeot|health:100/100|species:Pidgeot|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pidgeot,player-1,1|name:Razor Wind|noanim",
                "prepare|mon:Pidgeot,player-1,1|move:Razor Wind",
                "addvolatile|mon:Pidgeot,player-1,1|volatile:Razor Wind|from:Two Turn Move",
                "addvolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:Razor Wind",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pidgeot,player-1,1|name:Razor Wind",
                "removevolatile|mon:Pidgeot,player-1,1|volatile:Razor Wind|from:Razor Wind",
                "split|side:1",
                "damage|mon:Pidgeot,player-2,1|health:94/143",
                "damage|mon:Pidgeot,player-2,1|health:66/100",
                "removevolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:Two Turn Move",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn fly_grants_invulnerability() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            1035924651236077,
            two_pidgeot().unwrap(),
            two_pidgeot().unwrap(),
        )
        .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_lock_move_request = serde_json::from_str(
            r#"{
                "team_position": 0,
                "moves": [
                    {
                        "name": "Fly",
                        "id": "fly",
                        "pp": 0,
                        "max_pp": 0,
                        "disabled": false
                    }
                ],
                "trapped": true
            }"#,
        )
        .unwrap();
        assert_eq!(
            battle
                .request_for_player("player-1")
                .map(|req| if let Request::Turn(req) = req {
                    req.active.get(0).cloned()
                } else {
                    None
                })
                .flatten(),
            Some(expected_lock_move_request)
        );

        assert_error_message(
            battle.set_player_choice("player-1", "move 1"),
            "cannot move: Pidgeot does not have a move in slot 1",
        );
        assert_error_message(
            battle.set_player_choice("player-1", "switch 1"),
            "cannot switch: Pidgeot is trapped",
        );

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

        // Show Gust can hit Mons in Fly, and gains double power.
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

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
                "switch|player:player-1|position:1|name:Pidgeot|health:100/100|species:Pidgeot|level:50|gender:M",
                "switch|player:player-2|position:1|name:Pidgeot|health:100/100|species:Pidgeot|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Pidgeot,player-1,1|name:Fly|noanim",
                "prepare|mon:Pidgeot,player-1,1|move:Fly",
                "addvolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:Two Turn Move",
                "addvolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:Fly",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Tackle|target:Pidgeot,player-1,1",
                "miss|mon:Pidgeot,player-1,1",
                "move|mon:Pidgeot,player-1,1|name:Fly|target:Pidgeot,player-2,1",
                "removevolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:Fly",
                "split|side:1",
                "damage|mon:Pidgeot,player-2,1|health:79/143",
                "damage|mon:Pidgeot,player-2,1|health:56/100",
                "removevolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:Two Turn Move",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Pidgeot,player-1,1|name:Fly|noanim",
                "prepare|mon:Pidgeot,player-1,1|move:Fly",
                "addvolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:Two Turn Move",
                "addvolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:Fly",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Pidgeot,player-2,1|name:Gust|target:Pidgeot,player-1,1",
                "split|side:0",
                "damage|mon:Pidgeot,player-1,1|health:95/143",
                "damage|mon:Pidgeot,player-1,1|health:67/100",
                "move|mon:Pidgeot,player-1,1|name:Fly|target:Pidgeot,player-2,1",
                "removevolatile|mon:Pidgeot,player-1,1|volatile:Fly|from:Fly",
                "split|side:1",
                "damage|mon:Pidgeot,player-2,1|health:21/143",
                "damage|mon:Pidgeot,player-2,1|health:15/100",
                "removevolatile|mon:Pidgeot,player-1,1|volatile:Two Turn Move|from:Two Turn Move",
                "residual",
                "turn|turn:5"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
