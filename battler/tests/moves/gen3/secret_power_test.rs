use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        FieldEnvironment,
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
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn meditite() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Meditite",
                    "species": "Meditite",
                    "ability": "No Ability",
                    "moves": [
                        "Secret Power",
                        "Misty Terrain"
                    ],
                    "nature": "Hardy",
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
    environment: Option<FieldEnvironment>,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_field_environment(environment.unwrap_or_default())
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn secret_power_can_cause_paralysis_by_default() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, None, meditite().unwrap(), meditite().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Meditite,player-1,1|name:Secret Power|target:Meditite,player-2,1",
            "split|side:1",
            "damage|mon:Meditite,player-2,1|health:66/90",
            "damage|mon:Meditite,player-2,1|health:74/100",
            "status|mon:Meditite,player-2,1|status:Paralysis",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn secret_power_can_cause_drop_special_attack_in_misty_terrain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, None, meditite().unwrap(), meditite().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Meditite,player-1,1|name:Misty Terrain",
            "fieldstart|move:Misty Terrain",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Meditite,player-1,1|name:Secret Power|target:Meditite,player-2,1",
            "split|side:1",
            "damage|mon:Meditite,player-2,1|health:66/90",
            "damage|mon:Meditite,player-2,1|health:74/100",
            "unboost|mon:Meditite,player-2,1|stat:spa|by:1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn secret_power_changes_behavior_based_on_field_environment() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        Some(FieldEnvironment::Ice),
        meditite().unwrap(),
        meditite().unwrap(),
    )
    .unwrap();
    assert_eq!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Meditite,player-1,1|name:Secret Power|target:Meditite,player-2,1",
            "split|side:1",
            "damage|mon:Meditite,player-2,1|health:66/90",
            "damage|mon:Meditite,player-2,1|health:74/100",
            "status|mon:Meditite,player-2,1|status:Freeze",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
