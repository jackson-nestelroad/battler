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

#[test]
fn enforces_unique_abilities_by_default() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle_builder()
        .with_rule("Ability Clause")
        .build(&data)
        .unwrap();

    let mut bad_team = three_starters().unwrap();
    bad_team.members[1].ability = "Overgrow".to_owned();

    assert_matches::assert_matches!(battle.update_team("player-1", bad_team), Ok(()));

    assert_matches::assert_matches!(battle.validate_player("player-1"), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Ability Overgrow appears more than 1 time."), "{err:?}");
        });
    });
    assert_matches::assert_matches!(battle.start(), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Validation failed for Player 1: Ability Overgrow appears more than 1 time."), "{err:?}");
        });
    });

    assert_matches::assert_matches!(
        battle.update_team("player-1", three_starters().unwrap()),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.update_team("player-2", three_starters().unwrap()),
        Ok(())
    );

    assert_matches::assert_matches!(battle.validate_player("player-1"), Ok(()));
    assert_matches::assert_matches!(battle.start(), Ok(()));
}

#[test]
fn enforces_ability_limits() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle_builder()
        .with_rule("Ability Clause = 2")
        .build(&data)
        .unwrap();

    let mut bad_team = three_starters().unwrap();
    bad_team.members[0].ability = "Blaze".to_owned();
    bad_team.members[2].ability = "Blaze".to_owned();

    assert_matches::assert_matches!(battle.update_team("player-1", bad_team), Ok(()));

    assert_matches::assert_matches!(battle.validate_player("player-1"), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Ability Blaze appears more than 2 times."), "{err:?}");
        });
    });
    assert_matches::assert_matches!(battle.start(), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Validation failed for Player 1: Ability Blaze appears more than 2 times."), "{err:?}");
        });
    });

    let mut good_team = three_starters().unwrap();
    good_team.members[0].ability = "Blaze".to_owned();

    assert_matches::assert_matches!(battle.update_team("player-1", good_team.clone()), Ok(()));
    assert_matches::assert_matches!(battle.update_team("player-2", good_team), Ok(()));

    assert_matches::assert_matches!(battle.validate_player("player-1"), Ok(()));
    assert_matches::assert_matches!(battle.start(), Ok(()));
}

#[test]
fn fails_for_invalid_value() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    assert_matches::assert_matches!(
        make_battle_builder()
            .with_rule("Ability Clause = abc")
            .build(&data)
            .err(),
        Some(err) => {
            assert!(format!("{err:#}").contains("rule Ability Clause is invalid"), "{err:?}");
        }
    );
}
