#[cfg(test)]
mod flinch_test {
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

    fn rapidash() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
            "members": [
                {
                    "name": "Rapidash",
                    "species": "Rapidash",
                    "ability": "No Ability",
                    "moves": [
                        "Stomp"
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
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_base_damage_randomization(BattleEngineRandomizeBaseDamage::Max)
            .with_seed(77777)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn flinch_prevents_movement() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, rapidash().unwrap(), rapidash().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
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
                "switch|player:player-1|position:1|name:Rapidash|health:100/100|species:Rapidash|level:50|gender:M",
                "switch|player:player-2|position:1|name:Rapidash|health:100/100|species:Rapidash|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Rapidash,player-2,1|name:Stomp|target:Rapidash,player-1,1",
                "split|side:0",
                "damage|mon:Rapidash,player-1,1|health:83/125",
                "damage|mon:Rapidash,player-1,1|health:67/100",
                "move|mon:Rapidash,player-1,1|name:Stomp|target:Rapidash,player-2,1",
                "split|side:1",
                "damage|mon:Rapidash,player-2,1|health:83/125",
                "damage|mon:Rapidash,player-2,1|health:67/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Rapidash,player-1,1|name:Stomp|target:Rapidash,player-2,1",
                "split|side:1",
                "damage|mon:Rapidash,player-2,1|health:41/125",
                "damage|mon:Rapidash,player-2,1|health:33/100",
                "cant|mon:Rapidash,player-2,1|reason:Flinch",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
