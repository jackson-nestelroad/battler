#[cfg(test)]
mod doubles_damage_calculation_tests {
    use battler::{
        battle::{
            BattleType,
            CoreBattleEngineRandomizeBaseDamage,
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

    fn blastoise() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Blastoise",
                        "species": "Blastoise",
                        "ability": "Torrent",
                        "moves": [
                            "Surf"
                        ],
                        "nature": "Impish",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50,
                        "ivs": {
                            "hp": 31,
                            "atk": 31,
                            "def": 31,
                            "spa": 31,
                            "spd": 31,
                            "spe": 31
                        },
                        "evs": {
                            "hp": 252,
                            "def": 216,
                            "spd": 40
                        }
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn venusaur_charizard() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Venusaur",
                        "species": "Venusaur",
                        "ability": "Overgrow",
                        "moves": [
                            "Tackle",
                            "Giga Drain",
                            "Earthquake"
                        ],
                        "nature": "Serious",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50,
                        "ivs": {
                            "hp": 31,
                            "atk": 31,
                            "def": 31,
                            "spa": 31,
                            "spd": 31,
                            "spe": 31
                        },
                        "evs": {
                            "def": 4,
                            "spa": 252,
                            "spe": 252
                        }
                    },
                    {
                        "name": "Charizard",
                        "species": "Charizard",
                        "ability": "Blaze",
                        "moves": [
                            "Fire Blast",
                            "Flamethrower",
                            "Air Slash",
                            "Dragon Claw"
                        ],
                        "nature": "Timid",
                        "gender": "F",
                        "ball": "Normal",
                        "level": 50,
                        "ivs": {
                            "hp": 31,
                            "atk": 31,
                            "def": 31,
                            "spa": 31,
                            "spd": 31,
                            "spe": 31
                        },
                        "evs": {
                            "spa": 252,
                            "spd": 4,
                            "spe": 252
                        }
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn test_battle_builder(team_1: TeamData, team_2: TeamData) -> TestBattleBuilder {
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Doubles)
            .with_pass_allowed(true)
            .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .with_team("player-1", team_1)
            .with_team("player-2", team_2)
    }

    fn make_battle_with_max_damage(
        data: &dyn DataStore,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        test_battle_builder(team_1, team_2)
            .with_seed(0)
            .with_controlled_rng(true)
            .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
            .build(data)
    }

    fn make_battle_with_min_damage(
        data: &dyn DataStore,
        team_1: TeamData,
        team_2: TeamData,
    ) -> Result<PublicCoreBattle, Error> {
        test_battle_builder(team_1, team_2)
            .with_seed(0)
            .with_controlled_rng(true)
            .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
            .build(data)
    }

    // Venusaur: 15-18.
    // Charizard: 68-84.
    //
    // Venusaur (once Charizard has fainted): 21-24.
    #[test]
    fn blastoise_surfs_venusaur_and_charizard() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, blastoise().unwrap(), venusaur_charizard().unwrap())
                .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[

                "move|mon:Blastoise,player-1,1|name:Surf|spread:Venusaur,player-2,1;Charizard,player-2,2",
                "resisted|mon:Venusaur,player-2,1",
                "supereffective|mon:Charizard,player-2,2",
                "split|side:1",
                "damage|mon:Venusaur,player-2,1|health:137/155",
                "damage|mon:Venusaur,player-2,1|health:89/100",
                "split|side:1",
                "damage|mon:Charizard,player-2,2|health:69/153",
                "damage|mon:Charizard,player-2,2|health:46/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Blastoise,player-1,1|name:Surf|spread:Venusaur,player-2,1;Charizard,player-2,2",
                "resisted|mon:Venusaur,player-2,1",
                "supereffective|mon:Charizard,player-2,2",
                "split|side:1",
                "damage|mon:Venusaur,player-2,1|health:119/155",
                "damage|mon:Venusaur,player-2,1|health:77/100",
                "split|side:1",
                "damage|mon:Charizard,player-2,2|health:0",
                "damage|mon:Charizard,player-2,2|health:0",
                "faint|mon:Charizard,player-2,2",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Blastoise,player-1,1|name:Surf",
                "resisted|mon:Venusaur,player-2,1",
                "split|side:1",
                "damage|mon:Venusaur,player-2,1|health:95/155",
                "damage|mon:Venusaur,player-2,1|health:62/100",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, blastoise().unwrap(), venusaur_charizard().unwrap())
                .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

        let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
            r#"[
                "move|mon:Blastoise,player-1,1|name:Surf|spread:Venusaur,player-2,1;Charizard,player-2,2",
                "resisted|mon:Venusaur,player-2,1",
                "supereffective|mon:Charizard,player-2,2",
                "split|side:1",
                "damage|mon:Venusaur,player-2,1|health:140/155",
                "damage|mon:Venusaur,player-2,1|health:91/100",
                "split|side:1",
                "damage|mon:Charizard,player-2,2|health:85/153",
                "damage|mon:Charizard,player-2,2|health:56/100",
                "residual",
                "turn|turn:2",
                ["time"],
                "move|mon:Blastoise,player-1,1|name:Surf|spread:Venusaur,player-2,1;Charizard,player-2,2",
                "resisted|mon:Venusaur,player-2,1",
                "supereffective|mon:Charizard,player-2,2",
                "split|side:1",
                "damage|mon:Venusaur,player-2,1|health:125/155",
                "damage|mon:Venusaur,player-2,1|health:81/100",
                "split|side:1",
                "damage|mon:Charizard,player-2,2|health:17/153",
                "damage|mon:Charizard,player-2,2|health:12/100",
                "residual",
                "turn|turn:3",
                ["time"],
                "move|mon:Blastoise,player-1,1|name:Surf|spread:Venusaur,player-2,1;Charizard,player-2,2",
                "resisted|mon:Venusaur,player-2,1",
                "supereffective|mon:Charizard,player-2,2",
                "split|side:1",
                "damage|mon:Venusaur,player-2,1|health:110/155",
                "damage|mon:Venusaur,player-2,1|health:71/100",
                "split|side:1",
                "damage|mon:Charizard,player-2,2|health:0",
                "damage|mon:Charizard,player-2,2|health:0",
                "faint|mon:Charizard,player-2,2",
                "residual",
                "turn|turn:4"
            ]"#,
        )
        .unwrap();
        assert_logs_since_turn_eq(&battle, 1, &expected_logs);
    }
}
