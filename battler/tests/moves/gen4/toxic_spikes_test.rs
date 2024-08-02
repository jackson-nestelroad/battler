#[cfg(test)]
mod toxic_spikes_test {
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

    fn team() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Roserade",
                        "species": "Roserade",
                        "ability": "No Ability",
                        "moves": [
                            "Toxic Spikes"
                        ],
                        "nature": "Hardy",
                        "level": 50
                    },
                    {
                        "name": "Torterra",
                        "species": "Torterra",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "level": 50
                    },
                    {
                        "name": "Infernape",
                        "species": "Infernape",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "level": 50
                    },
                    {
                        "name": "Vespiquen",
                        "species": "Vespiquen",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "level": 50
                    },
                    {
                        "name": "Empoleon",
                        "species": "Empoleon",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "level": 50
                    },
                    {
                        "name": "Drapion",
                        "species": "Drapion",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "level": 50
                    },
                    {
                        "name": "Rampardos",
                        "species": "Rampardos",
                        "ability": "No Ability",
                        "moves": [],
                        "nature": "Hardy",
                        "level": 50,
                        "item": "Heavy-Duty Boots"
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
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Reverse)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
            .build(data)
    }

    #[test]
    fn toxic_spikes_poison_opposing_side_on_switch_in() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "pass;switch 2"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "switch 1;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Roserade,player-1,1|name:Toxic Spikes",
                "sidestart|side:1|move:Toxic Spikes",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-2|position:2|name:Infernape|health:100/100|species:Infernape|level:50|gender:F",
                "status|mon:Infernape,player-2,2|status:Poison",
                "move|mon:Roserade,player-1,1|name:Toxic Spikes",
                "sidestart|side:1|move:Toxic Spikes",
                "split|side:1",
                "damage|mon:Infernape,player-2,2|from:status:Poison|health:119/136",
                "damage|mon:Infernape,player-2,2|from:status:Poison|health:88/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "switch|player:player-2|position:1|name:Torterra|health:100/100|species:Torterra|level:50|gender:F",
                "status|mon:Torterra,player-2,1|status:Bad Poison",
                "move|mon:Roserade,player-1,1|name:Toxic Spikes|noanim",
                "fail|mon:Roserade,player-1,1",
                "split|side:1",
                "damage|mon:Infernape,player-2,2|from:status:Poison|health:102/136",
                "damage|mon:Infernape,player-2,2|from:status:Poison|health:75/100",
                "split|side:1",
                "damage|mon:Torterra,player-2,1|from:status:Bad Poison|health:146/155",
                "damage|mon:Torterra,player-2,1|from:status:Bad Poison|health:95/100",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn flying_types_avoid_toxic_spikes() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "switch 3;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Roserade,player-1,1|name:Toxic Spikes",
                "sidestart|side:1|move:Toxic Spikes",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-2|position:1|name:Vespiquen|health:100/100|species:Vespiquen|level:50|gender:F",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn steel_types_are_immune_to_toxic_spikes() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "switch 4;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Roserade,player-1,1|name:Toxic Spikes",
                "sidestart|side:1|move:Toxic Spikes",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-2|position:1|name:Empoleon|health:100/100|species:Empoleon|level:50|gender:F",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn poison_type_absorbs_toxic_spikes() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "switch 5;pass"),
            Ok(())
        );
        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "switch 2;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Roserade,player-1,1|name:Toxic Spikes",
                "sidestart|side:1|move:Toxic Spikes",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Roserade,player-1,1|name:Toxic Spikes",
                "sidestart|side:1|move:Toxic Spikes",
                "residual",
                "turn|turn:3",
                ["time"],
                "switch|player:player-2|position:1|name:Drapion|health:100/100|species:Drapion|level:50|gender:F",
                "sideend|side:1|move:Toxic Spikes|of:Drapion,player-2,1",
                "residual",
                "turn|turn:4",
                ["time"],
                "switch|player:player-2|position:1|name:Infernape|health:100/100|species:Infernape|level:50|gender:F",
                "residual",
                "turn|turn:5"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }

    #[test]
    fn heavy_duty_boots_avoid_toxic_spikes() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
        assert_eq!(
            battle.set_player_choice("player-2", "switch 6;pass"),
            Ok(())
        );

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Roserade,player-1,1|name:Toxic Spikes",
                "sidestart|side:1|move:Toxic Spikes",
                "residual",
                "turn|turn:2",
                ["time"],
                "switch|player:player-2|position:1|name:Rampardos|health:100/100|species:Rampardos|level:50|gender:F",
                "residual",
                "turn|turn:3"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
