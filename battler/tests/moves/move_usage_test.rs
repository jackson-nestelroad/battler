#[cfg(test)]
mod move_usage_tests {
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

    fn team_1() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn team_2() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Blastoise",
                        "species": "Blastoise",
                        "ability": "Torrent",
                        "moves": ["Tackle"],
                        "nature": "Modest",
                        "gender": "F",
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
            .with_seed(0)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
            .with_team("player-1", team_1()?)
            .with_team("player-2", team_2()?)
            .build(data)
    }

    #[test]
    fn moves_can_be_used() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data).unwrap();
        assert_eq!(battle.start(), Ok(()));

        // Three turns of the Mons attacking each other with Tackle.
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

        // Expected logs are simple.
        //
        // We don't check damage calculations since it does have a random factor.
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:1|name:Blastoise|health:100/100|species:Blastoise|level:50|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Tackle|target:Blastoise,player-2,1",
                "split|side:1",
                "damage|mon:Blastoise,player-2,1|health:125/139",
                "damage|mon:Blastoise,player-2,1|health:90/100",
                "move|mon:Blastoise,player-2,1|name:Tackle|target:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:124/140",
                "damage|mon:Venusaur,player-1,1|health:89/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Tackle|target:Blastoise,player-2,1",
                "split|side:1",
                "damage|mon:Blastoise,player-2,1|health:112/139",
                "damage|mon:Blastoise,player-2,1|health:81/100",
                "move|mon:Blastoise,player-2,1|name:Tackle|target:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:108/140",
                "damage|mon:Venusaur,player-1,1|health:78/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Tackle|target:Blastoise,player-2,1",
                "split|side:1",
                "damage|mon:Blastoise,player-2,1|health:100/139",
                "damage|mon:Blastoise,player-2,1|health:72/100",
                "move|mon:Blastoise,player-2,1|name:Tackle|target:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:93/140",
                "damage|mon:Venusaur,player-1,1|health:67/100",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
