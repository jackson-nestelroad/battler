#[cfg(test)]
mod mist_test {
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

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "No Ability",
                        "moves": [
                            "Cotton Spore",
                            "Tail Whip",
                            "Sand Attack",
                            "Double Team",
                            "Tackle"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Blastoise",
                        "species": "Blastoise",
                        "ability": "No Ability",
                        "moves": [
                            "Mist",
                            "Aromatic Mist",
                            "Superpower"
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
            .with_battle_type(BattleType::Doubles)
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
    fn mist_protects_user_side_from_stat_drops() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle =
            make_battle(&data, 5456456324231453212, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-1", "move 1;move 1,-1"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-1", "move 2,1;move 2,2"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-2", "move 3;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-1", "move 4,1;pass"),
            Ok(())
        );
        assert_eq!(
            battle.set_player_choice("player-2", "move 4,1;pass"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:2",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "switch|player:player-1|position:2|name:Blastoise|health:100/100|species:Blastoise|level:50|gender:M",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:50|gender:M",
                "switch|player:player-2|position:2|name:Blastoise|health:100/100|species:Blastoise|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Blastoise,player-1,2|name:Mist",
                "sidestart|side:0|what:Mist",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Cotton Spore|spread:Charizard,player-1,1;Blastoise,player-1,2",
                "activate|move:Mist|mon:Charizard,player-1,1",
                "activate|move:Mist|mon:Blastoise,player-1,2",
                "move|mon:Charizard,player-1,1|name:Cotton Spore|spread:Charizard,player-2,1;Blastoise,player-2,2",
                "unboost|mon:Charizard,player-2,1|stat:spe|by:2",
                "unboost|mon:Blastoise,player-2,2|stat:spe|by:2",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Charizard,player-1,1|name:Tail Whip|spread:Charizard,player-2,1;Blastoise,player-2,2",
                "unboost|mon:Charizard,player-2,1|stat:def|by:1",
                "unboost|mon:Blastoise,player-2,2|stat:def|by:1",
                "move|mon:Blastoise,player-1,2|name:Aromatic Mist|target:Charizard,player-1,1",
                "boost|mon:Charizard,player-1,1|stat:spd|by:1",
                "move|mon:Charizard,player-2,1|name:Tail Whip|spread:Charizard,player-1,1;Blastoise,player-1,2",
                "activate|move:Mist|mon:Charizard,player-1,1",
                "activate|move:Mist|mon:Blastoise,player-1,2",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Charizard,player-1,1|name:Sand Attack|target:Charizard,player-2,1",
                "unboost|mon:Charizard,player-2,1|stat:acc|by:1",
                "move|mon:Blastoise,player-1,2|name:Superpower|target:Blastoise,player-2,2",
                "split|side:1",
                "damage|mon:Blastoise,player-2,2|health:80/139",
                "damage|mon:Blastoise,player-2,2|health:58/100",
                "unboost|mon:Blastoise,player-1,2|stat:atk|by:1",
                "unboost|mon:Blastoise,player-1,2|stat:def|by:1",
                "move|mon:Charizard,player-2,1|name:Double Team|target:Charizard,player-2,1",
                "boost|mon:Charizard,player-2,1|stat:eva|by:1",
                "residual",
                "turn|turn:5",
                ["time"],
                "move|mon:Charizard,player-1,1|name:Tackle|target:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:109/138",
                "damage|mon:Charizard,player-2,1|health:79/100",
                "move|mon:Charizard,player-2,1|name:Tackle|target:Charizard,player-1,1|notarget",
                "miss|mon:Charizard,player-1,1",
                "sideend|side:0|what:Mist",
                "residual",
                "turn|turn:6",
                ["time"],
                "move|mon:Charizard,player-1,1|name:Tail Whip|spread:Blastoise,player-2,2",
                "miss|mon:Charizard,player-2,1",
                "unboost|mon:Blastoise,player-2,2|stat:def|by:1",
                "move|mon:Charizard,player-2,1|name:Tail Whip|spread:Charizard,player-1,1",
                "miss|mon:Blastoise,player-1,2",
                "unboost|mon:Charizard,player-1,1|stat:def|by:1",
                "residual",
                "turn|turn:7"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
