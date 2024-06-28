#[cfg(test)]
mod stab_test {
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

    fn squirtle() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Squirtle",
                        "species": "Squirtle",
                        "ability": "Torrent",
                        "moves": [
                            "Tackle",
                            "Aqua Jet"
                        ],
                        "nature": "Adamant",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 40
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn pikachu() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Pikachu",
                        "species": "Pikachu",
                        "ability": "Static",
                        "moves": [
                            "Quick Attack"
                        ],
                        "nature": "Bold",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 40
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn test_battle_builder(team_1: TeamData, team_2: TeamData) -> TestBattleBuilder {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .with_seed(0)
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
    }

    fn make_battle(
        data: &dyn DataStore,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        test_battle_builder(team_1, team_2).build(data)
    }

    #[test]
    fn stab_increases_damage() {
        // Tackle and Aqua Jet are both Physical moves with the same base damage, so STAB makes the
        // difference.
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, squirtle().unwrap(), pikachu().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
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
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:1",
                "start",
                "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:40|gender:F",
                "switch|player:player-2|position:1|name:Pikachu|health:100/100|species:Pikachu|level:40|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Squirtle,player-1,1",
                "split|side:0",
                "damage|mon:Squirtle,player-1,1|health:73/85",
                "damage|mon:Squirtle,player-1,1|health:86/100",
                "move|mon:Squirtle,player-1,1|name:Tackle|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:61/78",
                "damage|mon:Pikachu,player-2,1|health:79/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Pikachu,player-2,1|name:Quick Attack|target:Squirtle,player-1,1",
                "split|side:0",
                "damage|mon:Squirtle,player-1,1|health:62/85",
                "damage|mon:Squirtle,player-1,1|health:73/100",
                "move|mon:Squirtle,player-1,1|name:Aqua Jet|target:Pikachu,player-2,1",
                "split|side:1",
                "damage|mon:Pikachu,player-2,1|health:36/78",
                "damage|mon:Pikachu,player-2,1|health:47/100",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
