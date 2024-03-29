#[cfg(test)]
mod damage_calculation_tests {
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
        get_controlled_rng_for_battle,
        LogMatch,
        TestBattleBuilder,
    };

    fn venusaur() -> Result<TeamData, Error> {
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
                        "level": 100,
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
                    }
                ]
            }"#,
        )
        .wrap_error()
    }

    fn charizard() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
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
                        "level": 100,
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

    fn level_60_charizard() -> Result<TeamData, Error> {
        serde_json::from_str(
            r#"{
                "members": [
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
                        "level": 60,
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
            .with_battle_type(BattleType::Singles)
            .with_pass_allowed(true)
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

    // Damage: 31-37.
    #[test]
    fn venusaur_tackles_charizard() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Tackle|target:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:260/297",
                "damage|mon:Charizard,player-2,1|health:88/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Tackle|target:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:266/297",
                "damage|mon:Charizard,player-2,1|health:90/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 29-34.
    #[test]
    fn venusaur_giga_drains_charizard() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Charizard,player-2,1",
                "resisted|mon:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:263/297",
                "damage|mon:Charizard,player-2,1|health:89/100",
                "split|side:0",
                "heal|mon:Venusaur,player-1,1|from:drain|of:Charizard,player-2,1|health:301/301",
                "heal|mon:Venusaur,player-1,1|from:drain|of:Charizard,player-2,1|health:100/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Charizard,player-2,1",
                "resisted|mon:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:268/297",
                "damage|mon:Charizard,player-2,1|health:91/100",
                "split|side:0",
                "heal|mon:Venusaur,player-1,1|from:drain|of:Charizard,player-2,1|health:301/301",
                "heal|mon:Venusaur,player-1,1|from:drain|of:Charizard,player-2,1|health:100/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 44-52.
    #[test]
    fn venusaur_giga_drains_charizard_with_crit() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));
        let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
        rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Charizard,player-2,1",
                "resisted|mon:Charizard,player-2,1",
                "crit|mon:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:245/297",
                "damage|mon:Charizard,player-2,1|health:83/100",
                "split|side:0",
                "heal|mon:Venusaur,player-1,1|from:drain|of:Charizard,player-2,1|health:301/301",
                "heal|mon:Venusaur,player-1,1|from:drain|of:Charizard,player-2,1|health:100/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));
        let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
        rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

        assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Charizard,player-2,1",
                "resisted|mon:Charizard,player-2,1",
                "crit|mon:Charizard,player-2,1",
                "split|side:1",
                "damage|mon:Charizard,player-2,1|health:253/297",
                "damage|mon:Charizard,player-2,1|health:86/100",
                "split|side:0",
                "heal|mon:Venusaur,player-1,1|from:drain|of:Charizard,player-2,1|health:301/301",
                "heal|mon:Venusaur,player-1,1|from:drain|of:Charizard,player-2,1|health:100/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 0.
    #[test]
    fn venusaur_earthquakes_charizard() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Venusaur,player-1,1|name:Earthquake",
                "immune|mon:Charizard,player-2,1",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 320-378.
    #[test]
    fn charizard_fire_blasts_venusaur() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Fire Blast|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:0",
                "damage|mon:Venusaur,player-1,1|health:0",
                "faint|mon:Venusaur,player-1,1",
                "win|side:1"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Fire Blast|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:0",
                "damage|mon:Venusaur,player-1,1|health:0",
                "faint|mon:Venusaur,player-1,1",
                "win|side:1"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 260-308.
    #[test]
    fn charizard_flamethrowers_venusaur() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Flamethrower|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:0",
                "damage|mon:Venusaur,player-1,1|health:0",
                "faint|mon:Venusaur,player-1,1",
                "win|side:1"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Flamethrower|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:41/301",
                "damage|mon:Venusaur,player-1,1|health:14/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 218-258.
    #[test]
    fn charizard_air_slashes_venusaur() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Air Slash|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:43/301",
                "damage|mon:Venusaur,player-1,1|health:15/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Air Slash|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:83/301",
                "damage|mon:Venusaur,player-1,1|health:28/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 52-62.
    #[test]
    fn charizard_dragon_claws_venusaur() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Dragon Claw|target:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:239/301",
                "damage|mon:Venusaur,player-1,1|health:80/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Dragon Claw|target:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:249/301",
                "damage|mon:Venusaur,player-1,1|health:83/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 79-93.
    #[test]
    fn charizard_dragon_claws_venusaur_with_crit() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));
        let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
        rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Dragon Claw|target:Venusaur,player-1,1",
                "crit|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:208/301",
                "damage|mon:Venusaur,player-1,1|health:70/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));
        let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
        rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("player-2", "move 3"), Ok(()));

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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Dragon Claw|target:Venusaur,player-1,1",
                "crit|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:222/301",
                "damage|mon:Venusaur,player-1,1|health:74/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 390-462.
    #[test]
    fn charizard_flamethrowers_venusaur_with_crit() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));
        let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
        rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                 "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                 "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                 "turn|turn:1",
                 ["time"],
                 "move|mon:Charizard,player-2,1|name:Flamethrower|target:Venusaur,player-1,1",
                 "supereffective|mon:Venusaur,player-1,1",
                 "crit|mon:Venusaur,player-1,1",
                 "split|side:0",
                 "damage|mon:Venusaur,player-1,1|health:0",
                 "damage|mon:Venusaur,player-1,1|health:0",
                 "faint|mon:Venusaur,player-1,1",
                 "win|side:1"
             ]"#,
         )
         .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
        assert_eq!(battle.start(), Ok(()));
        let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
        rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:100|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Flamethrower|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "crit|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:0",
                "damage|mon:Venusaur,player-1,1|health:0",
                "faint|mon:Venusaur,player-1,1",
                "win|side:1"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }

    // Damage: 102-120.
    #[test]
    fn level_60_charizard_flamethrowers_venusaur() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        let mut battle =
            make_battle_with_max_damage(&data, venusaur().unwrap(), level_60_charizard().unwrap())
                .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:60|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Flamethrower|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:181/301",
                "damage|mon:Venusaur,player-1,1|health:61/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);

        let mut battle =
            make_battle_with_min_damage(&data, venusaur().unwrap(), level_60_charizard().unwrap())
                .unwrap();
        assert_eq!(battle.start(), Ok(()));

        assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
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
                "switch|player:player-1|position:1|name:Venusaur|health:100/100|species:Venusaur|level:100|gender:F",
                "switch|player:player-2|position:1|name:Charizard|health:100/100|species:Charizard|level:60|gender:F",
                "turn|turn:1",
                ["time"],
                "move|mon:Charizard,player-2,1|name:Flamethrower|target:Venusaur,player-1,1",
                "supereffective|mon:Venusaur,player-1,1",
                "split|side:0",
                "damage|mon:Venusaur,player-1,1|health:199/301",
                "damage|mon:Venusaur,player-1,1|health:67/100",
                "residual",
                "turn|turn:2"
            ]"#,
        )
        .unwrap();
        assert_new_logs_eq(&mut battle, &expected_logs);
    }
}
