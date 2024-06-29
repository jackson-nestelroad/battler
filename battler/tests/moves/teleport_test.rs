#[cfg(test)]
mod teleport_test {
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
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn houndoom() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Houndoom",
                        "species": "Houndoom",
                        "ability": "No Ability",
                        "moves": [
                            "Dark Pulse"
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

    fn two_ralts() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Ralts",
                        "species": "Ralts",
                        "ability": "No Ability",
                        "moves": [
                            "Teleport"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Ralts",
                        "species": "Ralts",
                        "ability": "No Ability",
                        "moves": [
                            "Teleport"
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
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn teleport_switches_user_out() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, two_ralts().unwrap(), houndoom().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "switch 0"), Ok(()));

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
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Ralts|health:100/100|species:Ralts|level:50|gender:M",
                "switch|player:player-2|position:1|name:Houndoom|health:100/100|species:Houndoom|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Ralts,player-1,1|name:Teleport|target:Ralts,player-1,1",
                ["time"],
                "switch|player:player-1|position:1|name:Ralts|health:100/100|species:Ralts|level:50|gender:M",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Houndoom,player-2,1|name:Dark Pulse|target:Ralts,player-1,1",
                "split|side:0",
                "damage|mon:Ralts,player-1,1|health:0",
                "damage|mon:Ralts,player-1,1|health:0",
                "faint|mon:Ralts,player-1,1",
                "residual",
                ["time"],
                "switch|player:player-1|position:1|name:Ralts|health:100/100|species:Ralts|level:50|gender:M",
                "turn|turn:3",
                ["time"],
                "move|mon:Ralts,player-1,1|name:Teleport|noanim",
                "fail|mon:Ralts,player-1,1",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
