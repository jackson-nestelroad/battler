use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    ValidationError,
    WrapResultError,
};
use battler_test_utils::{
    TestBattleBuilder,
    static_local_data_store,
};
use itertools::Itertools;

fn full_accuracy_sleep_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [
                        "Spore"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": [
                        "Gravity"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn low_accuracy_sleep_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [
                        "Sleep Powder"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": [
                        "Gravity"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn low_accuracy_sleep_no_gravity_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [
                        "Sleep Powder"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": [
                        "Ember"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn low_accuracy_sleep_gmax_orbeetle_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [
                        "Sleep Powder"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": [
                        "Ember"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Orbeetle",
                    "species": "Orbeetle",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50,
                    "gigantamax_factor": true
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .with_rule("Gravity Sleep Clause")
        .build(static_local_data_store())
}

#[test]
fn allows_full_accuracy_sleep_move() {
    let mut battle = make_battle(
        0,
        full_accuracy_sleep_team().unwrap(),
        full_accuracy_sleep_team().unwrap(),
    )
    .unwrap();

    assert_matches::assert_matches!(battle.validate_player("player-1"), Ok(()));
}

#[test]
fn disallows_low_accuracy_sleep_move() {
    let mut battle = make_battle(
        0,
        low_accuracy_sleep_team().unwrap(),
        low_accuracy_sleep_team().unwrap(),
    )
    .unwrap();

    assert_matches::assert_matches!(battle.validate_player("player-1"), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Low-accuracy sleep moves (Sleep Powder) cannot be used with Gravity."), "{err:?}");
        });
    });
}

#[test]
fn allows_low_accuracy_sleep_move_without_gravity() {
    let mut battle = make_battle(
        0,
        low_accuracy_sleep_no_gravity_team().unwrap(),
        low_accuracy_sleep_no_gravity_team().unwrap(),
    )
    .unwrap();

    assert_matches::assert_matches!(battle.validate_player("player-1"), Ok(()));
}

#[test]
fn disallows_low_accuracy_sleep_move_with_gmax_orbeetle() {
    let mut battle = make_battle(
        0,
        low_accuracy_sleep_gmax_orbeetle_team().unwrap(),
        low_accuracy_sleep_gmax_orbeetle_team().unwrap(),
    )
    .unwrap();

    assert_matches::assert_matches!(battle.validate_player("player-1"), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Low-accuracy sleep moves (Sleep Powder) cannot be used with G-Max Orbeetle."), "{err:?}");
        });
    });
}

#[test]
fn allows_low_accuracy_sleep_move_with_non_gmax_orbeetle() {
    let mut team = low_accuracy_sleep_gmax_orbeetle_team().unwrap();
    team.members[2].gigantamax_factor = false;
    let mut battle =
        make_battle(0, team, low_accuracy_sleep_gmax_orbeetle_team().unwrap()).unwrap();

    assert_matches::assert_matches!(battle.validate_player("player-1"), Ok(()));
}
