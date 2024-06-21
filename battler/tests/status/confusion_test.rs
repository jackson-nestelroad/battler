#[cfg(test)]
mod confusion_test {
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

    fn crobat() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Crobat",
                        "species": "Crobat",
                        "ability": "No Ability",
                        "moves": [
                            "Supersonic",
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
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Crobat|health:100/100|species:Crobat|level:50|gender:M",
                "switch|player:player-2|position:1|name:Crobat|health:100/100|species:Crobat|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Crobat,player-2,1|name:Supersonic|target:Crobat,player-1,1",
                "start|mon:Crobat,player-1,1|what:Confusion",
                "activate|mon:Crobat,player-1,1|condition:Confusion",
                "move|mon:Crobat,player-1,1|name:Supersonic|target:Crobat,player-2,1",
                "start|mon:Crobat,player-2,1|what:Confusion",
                "residual",
                "turn|turn:2",
                ["time"],
                "activate|mon:Crobat,player-1,1|condition:Confusion",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|from:Confusion|health:126/145",
                "damage|mon:Crobat,player-1,1|from:Confusion|health:87/100",
                "activate|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:107/145",
                "damage|mon:Crobat,player-1,1|health:74/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "activate|mon:Crobat,player-1,1|condition:Confusion",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|from:Confusion|health:89/145",
                "damage|mon:Crobat,player-1,1|from:Confusion|health:62/100",
                "activate|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:70/145",
                "damage|mon:Crobat,player-1,1|health:49/100",
                "residual",
                "turn|turn:4",
                ["time"],
                "activate|mon:Crobat,player-1,1|condition:Confusion",
                "move|mon:Crobat,player-1,1|name:Tackle|target:Crobat,player-2,1",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|health:126/145",
                "damage|mon:Crobat,player-2,1|health:87/100",
                "activate|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:51/145",
                "damage|mon:Crobat,player-1,1|health:36/100",
                "residual",
                "turn|turn:5",
                ["time"],
                "end|mon:Crobat,player-1,1|what:Confusion",
                "move|mon:Crobat,player-1,1|name:Tackle|target:Crobat,player-2,1",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|health:106/145",
                "damage|mon:Crobat,player-2,1|health:74/100",
                "activate|mon:Crobat,player-2,1|condition:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:30/145",
                "damage|mon:Crobat,player-1,1|health:21/100",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Crobat,player-1,1|name:Tackle|target:Crobat,player-2,1",
                "split|side:1",
                "damage|mon:Crobat,player-2,1|health:89/145",
                "damage|mon:Crobat,player-2,1|health:62/100",
                "end|mon:Crobat,player-2,1|what:Confusion",
                "move|mon:Crobat,player-2,1|name:Tackle|target:Crobat,player-1,1",
                "split|side:0",
                "damage|mon:Crobat,player-1,1|health:12/145",
                "damage|mon:Crobat,player-1,1|health:9/100",
                "residual",
                "turn|turn:7"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
