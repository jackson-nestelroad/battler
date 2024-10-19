use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    Error,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
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
        .with_team_validation(false)
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

// Damage: 31-37.
#[test]
fn venusaur_tackles_charizard() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:260/297",
            "damage|mon:Charizard,player-2,1|health:88/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:266/297",
            "damage|mon:Charizard,player-2,1|health:90/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 29-34.
#[test]
fn venusaur_giga_drains_charizard() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Charizard,player-2,1",
            "resisted|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:263/297",
            "damage|mon:Charizard,player-2,1|health:89/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Charizard,player-2,1",
            "resisted|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:268/297",
            "damage|mon:Charizard,player-2,1|health:91/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 44-52.
#[test]
fn venusaur_giga_drains_charizard_with_crit() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Charizard,player-2,1",
            "resisted|mon:Charizard,player-2,1",
            "crit|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:245/297",
            "damage|mon:Charizard,player-2,1|health:83/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Giga Drain|target:Charizard,player-2,1",
            "resisted|mon:Charizard,player-2,1",
            "crit|mon:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:253/297",
            "damage|mon:Charizard,player-2,1|health:86/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 0.
#[test]
fn venusaur_earthquakes_charizard() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Earthquake|noanim",
            "immune|mon:Charizard,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 320-378.
#[test]
fn charizard_fire_blasts_venusaur() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 260-308.
#[test]
fn charizard_flamethrowers_venusaur() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 218-258.
#[test]
fn charizard_air_slashes_venusaur() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 52-62.
#[test]
fn charizard_dragon_claws_venusaur() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Dragon Claw|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:239/301",
            "damage|mon:Venusaur,player-1,1|health:80/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Dragon Claw|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:249/301",
            "damage|mon:Venusaur,player-1,1|health:83/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 79-93.
#[test]
fn charizard_dragon_claws_venusaur_with_crit() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 390-462.
#[test]
fn charizard_flamethrowers_venusaur_with_crit() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), charizard().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Damage: 102-120.
#[test]
fn level_60_charizard_flamethrowers_venusaur() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur().unwrap(), level_60_charizard().unwrap())
            .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle =
        make_battle_with_min_damage(&data, venusaur().unwrap(), level_60_charizard().unwrap())
            .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Same as charizard_dragon_claws_venusaur, but -1 Atk vs. +4 Def.
// Damage: 12-15.
#[test]
fn attack_and_defense_modifiers_impact_physical_move_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut venusaur = venusaur().unwrap();
    venusaur.members[0].moves[0] = "Growl".to_owned();
    venusaur.members[0].moves[1] = "Iron Defense".to_owned();
    let charizard = charizard().unwrap();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur.clone(), charizard.clone()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Growl",
            "unboost|mon:Charizard,player-2,1|stat:atk|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Iron Defense|target:Venusaur,player-1,1",
            "boost|mon:Venusaur,player-1,1|stat:def|by:2",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Iron Defense|target:Venusaur,player-1,1",
            "boost|mon:Venusaur,player-1,1|stat:def|by:2",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Dragon Claw|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:286/301",
            "damage|mon:Venusaur,player-1,1|health:96/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle = make_battle_with_min_damage(&data, venusaur, charizard).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Growl",
            "unboost|mon:Charizard,player-2,1|stat:atk|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Iron Defense|target:Venusaur,player-1,1",
            "boost|mon:Venusaur,player-1,1|stat:def|by:2",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Iron Defense|target:Venusaur,player-1,1",
            "boost|mon:Venusaur,player-1,1|stat:def|by:2",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Dragon Claw|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:289/301",
            "damage|mon:Venusaur,player-1,1|health:97/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

// Same as level_60_charizard_flamethrowers_venusaur, but +2 SpA vs. -1 SpD.
// Damage: 294-348.
#[test]
fn special_attack_and_defense_modifiers_impact_special_move_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut venusaur = venusaur().unwrap();
    venusaur.members[0].moves[0] = "Calm Mind".to_owned();
    let mut charizard = level_60_charizard().unwrap();
    charizard.members[0].moves[0] = "Nasty Plot".to_owned();
    charizard.members[0].moves[2] = "Fake Tears".to_owned();

    let mut battle =
        make_battle_with_max_damage(&data, venusaur.clone(), charizard.clone()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Calm Mind|target:Venusaur,player-1,1",
            "boost|mon:Venusaur,player-1,1|stat:spa|by:1",
            "boost|mon:Venusaur,player-1,1|stat:spd|by:1",
            "move|mon:Charizard,player-2,1|name:Nasty Plot|target:Charizard,player-2,1",
            "boost|mon:Charizard,player-2,1|stat:spa|by:2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Fake Tears|target:Venusaur,player-1,1",
            "unboost|mon:Venusaur,player-1,1|stat:spd|by:2",
            "residual",
            "turn|turn:3",
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
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    let mut battle = make_battle_with_min_damage(&data, venusaur, charizard).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Calm Mind|target:Venusaur,player-1,1",
            "boost|mon:Venusaur,player-1,1|stat:spa|by:1",
            "boost|mon:Venusaur,player-1,1|stat:spd|by:1",
            "move|mon:Charizard,player-2,1|name:Nasty Plot|target:Charizard,player-2,1",
            "boost|mon:Charizard,player-2,1|stat:spa|by:2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Fake Tears|target:Venusaur,player-1,1",
            "unboost|mon:Venusaur,player-1,1|stat:spd|by:2",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Flamethrower|target:Venusaur,player-1,1",
            "supereffective|mon:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:7/301",
            "damage|mon:Venusaur,player-1,1|health:3/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
