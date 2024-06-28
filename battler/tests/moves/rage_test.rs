#[cfg(test)]
mod rage_test {
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

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Gyarados",
                        "species": "Gyarados",
                        "ability": "No Ability",
                        "moves": [
                            "Rage",
                            "Tackle",
                            "Fury Attack"
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
    fn rage_increases_attack_on_hit() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 111111, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
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
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Gyarados|health:100/100|species:Gyarados|level:50|gender:M",
                "switch|player:player-2|position:1|name:Gyarados|health:100/100|species:Gyarados|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Gyarados,player-2,1|name:Tackle|target:Gyarados,player-1,1",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:130/155",
                "damage|mon:Gyarados,player-1,1|health:84/100",
                "move|mon:Gyarados,player-1,1|name:Rage|target:Gyarados,player-2,1",
                "crit|mon:Gyarados,player-2,1",
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|health:136/155",
                "damage|mon:Gyarados,player-2,1|health:88/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Gyarados,player-2,1|name:Tackle|target:Gyarados,player-1,1",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:106/155",
                "damage|mon:Gyarados,player-1,1|health:69/100",
                "boost|mon:Gyarados,player-1,1|stat:atk|by:1",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Gyarados,player-2,1|name:Fury Attack|target:Gyarados,player-1,1",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:96/155",
                "damage|mon:Gyarados,player-1,1|health:62/100",
                "boost|mon:Gyarados,player-1,1|stat:atk|by:1",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:85/155",
                "damage|mon:Gyarados,player-1,1|health:55/100",
                "boost|mon:Gyarados,player-1,1|stat:atk|by:1",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:75/155",
                "damage|mon:Gyarados,player-1,1|health:49/100",
                "boost|mon:Gyarados,player-1,1|stat:atk|by:1",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:65/155",
                "damage|mon:Gyarados,player-1,1|health:42/100",
                "boost|mon:Gyarados,player-1,1|stat:atk|by:1",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:54/155",
                "damage|mon:Gyarados,player-1,1|health:35/100",
                "boost|mon:Gyarados,player-1,1|stat:atk|by:1",
                "hitcount|hits:5",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Gyarados,player-2,1|name:Fury Attack|target:Gyarados,player-1,1",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:44/155",
                "damage|mon:Gyarados,player-1,1|health:29/100",
                "boost|mon:Gyarados,player-1,1|stat:atk|by:0",
                "split|side:0",
                "damage|mon:Gyarados,player-1,1|health:34/155",
                "damage|mon:Gyarados,player-1,1|health:22/100",
                "boost|mon:Gyarados,player-1,1|stat:atk|by:0",
                "hitcount|hits:2",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Gyarados,player-1,1|name:Rage|target:Gyarados,player-2,1",
                "split|side:1",
                "damage|mon:Gyarados,player-2,1|health:87/155",
                "damage|mon:Gyarados,player-2,1|health:57/100",
                "residual",
                "turn|turn:6"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
