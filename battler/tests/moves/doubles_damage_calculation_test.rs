#[cfg(test)]
mod doubles_damage_calculation_tests {
    use battler::{
        battle::{
            Battle,
            BattleEngineRandomizeBaseDamage,
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
            .with_speed_sort_tie_resolution(BattleEngineSpeedSortTieResolution::Keep)
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
            .with_base_damage_randomization(BattleEngineRandomizeBaseDamage::Max)
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
            .with_base_damage_randomization(BattleEngineRandomizeBaseDamage::Min)
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
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Blastoise|health:100/100|species:Blastoise|level:50|gender:F",
                "switch|player:player-2|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Charizard|health:100/100|species:Charizard|level:50|gender:F",
                "turn|turn:1",
                ["time"],
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
        assert_new_logs_eq(&mut battle, &expected_logs);

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
                "info|battletype:Doubles",
                "side|id:0|name:Side 1",
                "side|id:1|name:Side 2",
                "player|id:player-1|name:Player 1|side:0|position:0",
                "player|id:player-2|name:Player 2|side:1|position:0",
                ["time"],
                "teamsize|player:player-1|size:1",
                "teamsize|player:player-2|size:2",
                "start",
                "switch|player:player-1|position:1|name:Blastoise|health:100/100|species:Blastoise|level:50|gender:F",
                "switch|player:player-2|position:1|name:Venusaur|health:100/100|species:Venusaur|level:50|gender:F",
                "switch|player:player-2|position:2|name:Charizard|health:100/100|species:Charizard|level:50|gender:F",
                "turn|turn:1",
                ["time"],
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
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
