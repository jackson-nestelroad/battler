#[cfg(test)]
mod must_recharge_test {
    use battler::{
        battle::{
            Battle,
            BattleEngineSpeedSortTieResolution,
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

    fn two_venusaur() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "No Ability",
                        "moves": [
                            "Hyper Beam",
                            "Tackle"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "No Ability",
                        "moves": [
                            "Hyper Beam",
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
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(1087134089137400)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn recharge_moves_require_recharge_turn() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, two_venusaur().unwrap(), two_venusaur().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_lock_move_request = serde_json::from_str(
            r#"{
                "team_position": 0,
                "moves": [
                    {
                        "name": "Recharge",
                        "id": "recharge",
                        "pp": 0,
                        "max_pp": 0,
                        "target": "User",
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
            "cannot move: Venusaur does not have a move in slot 1",
        );

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Hyper Beam|target:Venusaur,player-2,1",
                "split|side:1",
                "damage|mon:Venusaur,player-2,1|health:75/140",
                "damage|mon:Venusaur,player-2,1|health:54/100",
                "mustrecharge|mon:Venusaur,player-1,1",
                "residual",
                "turn|turn:2",
                ["time"],
                "cant|mon:Venusaur,player-1,1|reason:Must Recharge",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Tackle|target:Venusaur,player-2,1",
                "split|side:1",
                "damage|mon:Venusaur,player-2,1|health:57/140",
                "damage|mon:Venusaur,player-2,1|health:41/100",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
