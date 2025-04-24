use anyhow::Result;
use battler::{
    error::ValidationError,
    BattleType,
    LocalDataStore,
    TeamData,
    WrapResultError,
};
use battler_test_utils::TestBattleBuilder;
use itertools::Itertools;

fn make_battle_builder() -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
        .with_team_validation(false)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
}

fn three_starters() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": ["Tackle"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "Blaze",
                    "moves": ["Scratch"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "Torrent",
                    "moves": ["Tackle"],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn three_water_mons() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "Torrent",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Omanyte",
                    "species": "Omanyte",
                    "ability": "Swift Swim",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Poliwrath",
                    "species": "Poliwrath",
                    "ability": "Damp",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

#[test]
fn enforces_mono_type() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle_builder()
        .with_rule("Force Mono Type = Water")
        .build(&data)
        .unwrap();

    assert_matches::assert_matches!(
        battle.update_team("player-1", three_starters().unwrap()),
        Ok(())
    );

    assert_matches::assert_matches!(battle.validate_player("player-1"), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Bulbasaur is not Water type."), "{err:?}");
            assert!(err.problems().contains(&"Charmander is not Water type."), "{err:?}");
        });
    });
    assert_matches::assert_matches!(battle.start(), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Validation failed for Player 1: Bulbasaur is not Water type."), "{err:?}");
            assert!(err.problems().contains(&"Validation failed for Player 1: Charmander is not Water type."), "{err:?}");
        });
    });

    assert_matches::assert_matches!(
        battle.update_team("player-1", three_water_mons().unwrap()),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.update_team("player-2", three_water_mons().unwrap()),
        Ok(())
    );

    assert_matches::assert_matches!(battle.validate_player("player-1"), Ok(()));
    assert_matches::assert_matches!(battle.start(), Ok(()));
}

#[test]
fn fails_for_missing_value() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    assert_matches::assert_matches!(
        make_battle_builder()
            .with_rule("Force Mono Type")
            .build(&data)
            .err(),
        Some(err) => {
            assert!(format!("{err:#}").contains("rule Force Mono Type is invalid"), "{err:?}");
        }
    );
}
