#[cfg(test)]
mod partially_trapped_test {
    use battler::{
        battle::{
            Battle,
            BattleEngineSpeedSortTieResolution,
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
        assert_error_message,
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn two_gyarados() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Gyarados",
                        "species": "Gyarados",
                        "ability": "No Ability",
                        "moves": [
                            "Bind"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Gyarados",
                        "species": "Gyarados",
                        "ability": "No Ability",
                        "moves": [
                            "Bind"
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
            .with_seed(0)
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
    fn bind_partially_traps_target() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, two_gyarados().unwrap(), two_gyarados().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        assert_error_message(
            battle.set_player_choice("player-2", "switch 1"),
            "cannot switch: Gyarados is trapped",
        );

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Gyarados|health:100/100|species:Gyarados|level:50|gender:M",
                "switch|player:player-2|position:1|name:Gyarados|health:100/100|species:Gyarados|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Gyarados,player-1,1|name:Bind|target:Gyarados,player-2,1",
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|health:144/155",
                "damage|mon:Gyarados,player-2,1|health:93/100",
                "activate|mon:Gyarados,player-2,1|move:Bind|of:Gyarados,player-1,1",
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:125/155",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:81/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:106/155",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:69/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:87/155",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:57/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:68/155",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:44/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "end|mon:Gyarados,player-2,1|move:Bind",
                "residual",
                "turn|turn:6",
                ["time"],
                "residual",
                "turn|turn:7"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn bind_ends_when_user_switches() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, two_gyarados().unwrap(), two_gyarados().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Gyarados|health:100/100|species:Gyarados|level:50|gender:M",
                "switch|player:player-2|position:1|name:Gyarados|health:100/100|species:Gyarados|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Gyarados,player-1,1|name:Bind|target:Gyarados,player-2,1",
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|health:144/155",
                "damage|mon:Gyarados,player-2,1|health:93/100",
                "activate|mon:Gyarados,player-2,1|move:Bind|of:Gyarados,player-1,1",
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:125/155",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:81/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:106/155",
                "damage|mon:Gyarados,player-2,1|from:move:Bind|health:69/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "switch|player:player-1|position:1|name:Gyarados|health:100/100|species:Gyarados|level:50|gender:M",
                "end|mon:Gyarados,player-2,1|move:Bind|silent",
                "residual",
                "turn|turn:4",
                ["time"],
                "residual",
                "turn|turn:5"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
