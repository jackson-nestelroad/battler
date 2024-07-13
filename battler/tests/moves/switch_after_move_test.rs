#[cfg(test)]
mod switch_after_move_test {
    use battler::{
        battle::{
            BattleType,
            CoreBattleEngineSpeedSortTieResolution,
            PublicCoreBattle,
        },
        common::Error,
        dex::{
            DataStore,
            LocalDataStore,
        },
        teams::TeamData,
    };
    use battler_test_utils::{
        assert_error_message_contains,
        assert_new_logs_eq,
        LogMatch,
        TestBattleBuilder,
    };

    fn make_battle(
        data: &dyn DataStore,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(555432123456)
            .with_team_validation(false)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn move_can_force_switch_random_target() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "No Ability",
                        "moves": [
                            "Tackle",
                            "Roar"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Charmander",
                        "species": "Charmander",
                        "ability": "No Ability",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Squirtle",
                        "species": "Squirtle",
                        "ability": "No Ability",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, team.clone(), team).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:89/105",
                "damage|mon:Bulbasaur,player-1,1|health:85/100",
                "move|mon:Bulbasaur,player-1,1|name:Roar|target:Bulbasaur,player-2,1",
                "drag|player:player-2|position:1|name:Squirtle|health:100/100|species:Squirtle|level:50|gender:M",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Squirtle,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:70/105",
                "damage|mon:Bulbasaur,player-1,1|health:67/100",
                "move|mon:Bulbasaur,player-1,1|name:Roar|target:Squirtle,player-2,1",
                "drag|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:53/105",
                "damage|mon:Bulbasaur,player-1,1|health:51/100",
                "move|mon:Bulbasaur,player-1,1|name:Roar|target:Bulbasaur,player-2,1",
                "drag|player:player-2|position:1|name:Squirtle|health:100/100|species:Squirtle|level:50|gender:M",
                "residual",
                "turn|turn:4",
                ["time"],
                "move|mon:Squirtle,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:35/105",
                "damage|mon:Bulbasaur,player-1,1|health:34/100",
                "move|mon:Bulbasaur,player-1,1|name:Roar|target:Squirtle,player-2,1",
                "drag|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:50|gender:M",
                "residual",
                "turn|turn:5"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    #[test]
    fn move_can_switch_user() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let team: TeamData = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulbasaur",
                        "species": "Bulbasaur",
                        "ability": "No Ability",
                        "moves": [
                            "Tackle",
                            "U-turn"
                        ],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Charmander",
                        "species": "Charmander",
                        "ability": "No Ability",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    },
                    {
                        "name": "Squirtle",
                        "species": "Squirtle",
                        "ability": "No Ability",
                        "moves": ["Tackle"],
                        "nature": "Hardy",
                        "gender": "M",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut battle = make_battle(&data, team.clone(), team).unwrap();

        assert_eq!(battle.start(), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_error_message_contains(
            battle.set_player_choice("player-2", "switch 2"),
            "you cannot do anything",
        );
        assert_eq!(battle.set_player_choice("player-1", "switch 2"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "info|battletype:Singles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:3",
                "teamsize|player:player-2|size:3",
                "start",
                "switch|player:player-1|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:50|gender:M",
                "turn|turn:1",
                ["time"],
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "split|side:0",
                "damage|mon:Bulbasaur,player-1,1|health:89/105",
                "damage|mon:Bulbasaur,player-1,1|health:85/100",
                "move|mon:Bulbasaur,player-1,1|name:U-turn|target:Bulbasaur,player-2,1",
                "split|side:1",
                "damage|mon:Bulbasaur,player-2,1|health:74/105",
                "damage|mon:Bulbasaur,player-2,1|health:71/100",
                ["time"],
                "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:50|gender:M",
                "residual",
                "turn|turn:2"
            ]"#).unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
