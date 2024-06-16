#[cfg(test)]
mod toxic_test {
    use battler::{
        battle::{
            Battle,
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

    fn venusaur() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "No Ability",
                        "moves": [
                            "Toxic"
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

    fn charizard() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "No Ability",
                        "moves": [
                            "Toxic"
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

    fn two_charizards() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "No Ability",
                        "moves": [
                            "Toxic"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "No Ability",
                        "moves": [
                            "Toxic"
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

    fn steelix() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Steelix",
                        "species": "Steelix",
                        "ability": "No Ability",
                        "moves": [],
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
            .with_seed(1234566456456)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn toxic_applies_increasing_residual_damage() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
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
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Toxic|target:Charizard,player-2,1",
                "status|mon:Charizard,player-2,1|status:Bad Poison",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:130/138",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:95/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Toxic|target:Charizard,player-2,1",
                "fail|mon:Venusaur,player-1,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:113/138",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:82/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "split|side:1",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:88/138",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:64/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "split|side:1",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:54/138",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:40/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "split|side:1",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:11/138",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:8/100",
                "residual",
                "turn|turn:6",
                ["time"],
                "split|side:1",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:0",
                "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:0",
                "residual",
                "faint|mon:Charizard,player-2,1",
                "win|side:0"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn poison_types_resist_toxic() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Toxic|target:Venusaur,player-1,1",
                "immune|mon:Venusaur,player-1,1",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn steel_types_resist_poison() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, steelix().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

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
                "switch|player:player-1|position:1|name:Steelix|health:100/100|species:Steelix|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Toxic|target:Steelix,player-1,1",
                "immune|mon:Steelix,player-1,1",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn switch_out_resets_toxic_state() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, venusaur().unwrap(), two_charizards().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
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
            "teamsize|player:player-1|size:1",
            "teamsize|player:player-2|size:2",
            "start",
            "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:M",
            "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
            "turn|turn:1",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Toxic|target:Charizard,player-2,1",
            "status|mon:Charizard,player-2,1|status:Bad Poison",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:130/138",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:95/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:113/138",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:82/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:1",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:88/138",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:64/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:1",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:54/138",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:40/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
            "residual",
            "turn|turn:6",
            ["time"],
            "switch|player:player-2|position:1|name:Charizard|health:40/100|status:Bad Poison|species:Charizard|level:50|gender:M",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:46/138",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:34/100",
            "residual",
            "turn|turn:7",
            ["time"],
            "split|side:1",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:29/138",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:22/100",
            "residual",
            "turn|turn:8",
            ["time"],
            "split|side:1",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:4/138",
            "damage|mon:Charizard,player-2,1|from:status:Bad Poison|health:3/100",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
