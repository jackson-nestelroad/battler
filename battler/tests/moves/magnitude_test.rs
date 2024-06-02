#[cfg(test)]
mod magnitude_test {
    use battler::{
        battle::{
            Battle,
            BattleEngineRandomizeBaseDamage,
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

    fn sandslash() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
            "members": [
                {
                    "name": "Sandslash",
                    "species": "Sandslash",
                    "ability": "No Ability",
                    "moves": [
                        "Magnitude",
                        "Recover"
                    ],
                    "pp_boosts": [0, 3],
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

    fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_team_validation(false)
            .with_base_damage_randomization(BattleEngineRandomizeBaseDamage::Max)
            .with_seed(204759285930)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", sandslash()?)
            .with_team("player-2", sandslash()?)
            .build(data)
    }

    #[test]
    fn magnitude_randomly_sets_base_power() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

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
                "switch|player:player-1|position:1|name:Sandslash|health:100/100|species:Sandslash|level:50|gender:M",
                "switch|player:player-2|position:1|name:Sandslash|health:100/100|species:Sandslash|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
                "fail|mon:Sandslash,player-2,1|what:heal",
                "move|mon:Sandslash,player-1,1|name:Magnitude",
                "activate|move:Magnitude|magnitude:7",
                "split|side:1",
                "damage|mon:Sandslash,player-2,1|health:90/135",
                "damage|mon:Sandslash,player-2,1|health:67/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Sandslash,player-1,1|name:Magnitude",
                "activate|move:Magnitude|magnitude:4",
                "split|side:1",
                "damage|mon:Sandslash,player-2,1|health:81/135",
                "damage|mon:Sandslash,player-2,1|health:60/100",
                "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
                "split|side:1",
                "heal|mon:Sandslash,player-2,1|health:135/135",
                "heal|mon:Sandslash,player-2,1|health:100/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Sandslash,player-1,1|name:Magnitude",
                "activate|move:Magnitude|magnitude:7",
                "split|side:1",
                "damage|mon:Sandslash,player-2,1|health:90/135",
                "damage|mon:Sandslash,player-2,1|health:67/100",
                "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
                "split|side:1",
                "heal|mon:Sandslash,player-2,1|health:135/135",
                "heal|mon:Sandslash,player-2,1|health:100/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
                "fail|mon:Sandslash,player-2,1|what:heal",
                "move|mon:Sandslash,player-1,1|name:Magnitude",
                "activate|move:Magnitude|magnitude:4",
                "split|side:1",
                "damage|mon:Sandslash,player-2,1|health:126/135",
                "damage|mon:Sandslash,player-2,1|health:94/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
                "split|side:1",
                "heal|mon:Sandslash,player-2,1|health:135/135",
                "heal|mon:Sandslash,player-2,1|health:100/100",
                "move|mon:Sandslash,player-1,1|name:Magnitude",
                "activate|move:Magnitude|magnitude:10",
                "split|side:1",
                "damage|mon:Sandslash,player-2,1|health:42/135",
                "damage|mon:Sandslash,player-2,1|health:32/100",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Sandslash,player-2,1|name:Recover|target:Sandslash,player-2,1",
                "split|side:1",
                "heal|mon:Sandslash,player-2,1|health:110/135",
                "heal|mon:Sandslash,player-2,1|health:82/100",
                "move|mon:Sandslash,player-1,1|name:Magnitude",
                "activate|move:Magnitude|magnitude:5",
                "split|side:1",
                "damage|mon:Sandslash,player-2,1|health:89/135",
                "damage|mon:Sandslash,player-2,1|health:66/100",
                "residual",
                "turn|turn:7"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
