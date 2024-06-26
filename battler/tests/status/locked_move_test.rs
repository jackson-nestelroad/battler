#[cfg(test)]
mod locked_move_test {
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

    fn blissey() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Blissey",
                        "species": "Blissey",
                        "ability": "No Ability",
                        "moves": [
                            "Thrash",
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
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn thrash_locks_move_and_confuses_user() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(
            &data,
            20598204958240985,
            blissey().unwrap(),
            blissey().unwrap(),
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
                        "name": "Thrash",
                        "id": "thrash",
                        "pp": 0,
                        "max_pp": 0,
                        "disabled": false
                    }
                ]
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
            "cannot move: Blissey does not have a move in slot 1",
        );

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
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
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Blissey|health:100/100|species:Blissey|level:50|gender:M",
                "switch|player:player-2|position:1|name:Blissey|health:100/100|species:Blissey|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Blissey,player-1,1|name:Thrash|target:Blissey,player-2,1",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|health:237/315",
                "damage|mon:Blissey,player-2,1|health:76/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Blissey,player-1,1|name:Thrash|target:Blissey,player-2,1",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|health:168/315",
                "damage|mon:Blissey,player-2,1|health:54/100",
                "start|mon:Blissey,player-1,1|condition:Confusion|fatigue",
                "move|mon:Blissey,player-2,1|name:Tackle|target:Blissey,player-1,1",
                "split|side:0",
                "damage|mon:Blissey,player-1,1|health:288/315",
                "damage|mon:Blissey,player-1,1|health:92/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "activate|mon:Blissey,player-1,1|condition:Confusion",
                "move|mon:Blissey,player-1,1|name:Tackle|target:Blissey,player-2,1",
                "split|side:1",
                "damage|mon:Blissey,player-2,1|health:141/315",
                "damage|mon:Blissey,player-2,1|health:45/100",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
