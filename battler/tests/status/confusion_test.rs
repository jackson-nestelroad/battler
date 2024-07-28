#[cfg(test)]
mod confusion_test {
    use battler::{
        battle::{
            BattleType,
            CoreBattleEngineSpeedSortTieResolution,
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
        assert_logs_since_turn_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn crobat() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Crobat",
                        "species": "Crobat",
                        "ability": "No Ability",
                        "moves": [
                            "Confuse Ray",
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
            .with_seed(954225157957056)
            .with_team_validation(false)
            .with_pass_allowed(true)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn confusion_can_hurt_user_and_wears_off_naturally() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, crobat().unwrap(), crobat().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Crobat,player-2,1|name:Confuse Ray|target:Crobat,player-1,1",
                "start|mon:Crobat,player-1,1|condition:Confusion",
                "activate|mon:Crobat,player-1,1|condition:Confusion",
                "move|mon:Crobat,player-1,1|name:Confuse Ray|target:Crobat,player-2,1",
                "start|mon:Crobat,player-2,1|condition:Confusion",
                "residual",
                "turn|turn:2",
                ["time"],
                "activate|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:127/145",
                "damage|mon:Crobat,player-1,1|health:88/100",
                "activate|mon:Crobat,player-1,1|condition:Confusion",
                "move|mon:Crobat,player-1,1|name:Tackle|target:Crobat,player-2,1",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|health:127/145",
                "damage|mon:Crobat,player-2,1|health:88/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "activate|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:108/145",
                "damage|mon:Crobat,player-1,1|health:75/100",
                "activate|mon:Crobat,player-1,1|condition:Confusion",
                "move|mon:Crobat,player-1,1|name:Tackle|target:Crobat,player-2,1",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|health:107/145",
                "damage|mon:Crobat,player-2,1|health:74/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "activate|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:91/145",
                "damage|mon:Crobat,player-1,1|health:63/100",
                "activate|mon:Crobat,player-1,1|condition:Confusion",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|from:Confusion|health:73/145",
                "damage|mon:Crobat,player-1,1|from:Confusion|health:51/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "activate|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:55/145",
                "damage|mon:Crobat,player-1,1|health:38/100",
                "end|mon:Crobat,player-1,1|condition:Confusion",
                "move|mon:Crobat,player-1,1|name:Tackle|target:Crobat,player-2,1",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|health:89/145",
                "damage|mon:Crobat,player-2,1|health:62/100",
                "residual",
                "turn|turn:6",
                ["time"],
                "end|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:37/145",
                "damage|mon:Crobat,player-1,1|health:26/100",
                "move|mon:Crobat,player-1,1|name:Tackle|target:Crobat,player-2,1",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|health:69/145",
                "damage|mon:Crobat,player-2,1|health:48/100",
                "residual",
                "turn|turn:7"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
