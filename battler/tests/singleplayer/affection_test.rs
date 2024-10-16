use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn pikachu() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50,
                    "friendship": 255
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn eevee() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "No Ability",
                    "moves": [
                        "Earthquake",
                        "Toxic",
                        "Pin Missile"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50,
                    "friendship": 255
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_trainer_singles_battle(
    data: &dyn DataStore,
    seed: u64,
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
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_player_to_side_2("trainer", "Trainer")
        .with_team("protagonist", team_1)
        .with_team("trainer", team_2)
        .build(data)
}

#[test]
fn affection_cures_status() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_trainer_singles_battle(&data, 0, pikachu().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,trainer,1|name:Toxic|target:Pikachu,protagonist,1",
            "status|mon:Pikachu,protagonist,1|status:Bad Poison",
            "curestatus|mon:Pikachu,protagonist,1|status:Bad Poison|from:Affection",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn affection_survives_hit() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_trainer_singles_battle(&data, 0, pikachu().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,trainer,1|name:Earthquake",
            "supereffective|mon:Pikachu,protagonist,1",
            "activate|mon:Pikachu,protagonist,1|condition:Affection|tough",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:1/95",
            "damage|mon:Pikachu,protagonist,1|health:2/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn affection_survives_multiple_hits() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_trainer_singles_battle(&data, 0, pikachu().unwrap(), eevee().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([
        (4, 0),
        (5, 18),
        (6, 0),
        (7, 0),
        (9, 0),
        (11, 0),
        (15, 0),
        (18, 0),
        (19, 0),
        (21, 99),
    ]);

    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("protagonist", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("trainer", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,trainer,1|name:Pin Missile|target:Pikachu,protagonist,1",
            "crit|mon:Pikachu,protagonist,1",
            "activate|mon:Pikachu,protagonist,1|condition:Affection|tough",
            "animatemove|mon:Eevee,trainer,1|name:Pin Missile|target:Pikachu,protagonist,1",
            "activate|mon:Pikachu,protagonist,1|condition:Affection|tough",
            "animatemove|mon:Eevee,trainer,1|name:Pin Missile|target:Pikachu,protagonist,1",
            "activate|mon:Pikachu,protagonist,1|condition:Affection|tough",
            "animatemove|mon:Eevee,trainer,1|name:Pin Missile|target:Pikachu,protagonist,1",
            "activate|mon:Pikachu,protagonist,1|condition:Affection|tough",
            "animatemove|mon:Eevee,trainer,1|name:Pin Missile|target:Pikachu,protagonist,1",
            "crit|mon:Pikachu,protagonist,1",
            "split|side:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "damage|mon:Pikachu,protagonist,1|health:0",
            "faint|mon:Pikachu,protagonist,1",
            "hitcount|hits:5",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
