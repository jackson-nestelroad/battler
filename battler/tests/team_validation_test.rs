use battler::{
    error::ValidationError,
    BattleType,
    Error,
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
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
}

fn three_starters() -> Result<TeamData, Error> {
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
fn validates_empty_side() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    assert_matches::assert_matches!(
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .add_player_to_side_1("player-1", "Player 1")
            .build(&data)
            .err(),
        Some(err) => {
            assert!(err.full_description().contains("Side 2 has no players"), "{err:?}");
        }
    );
}

#[test]
fn validates_players_per_side() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    assert_matches::assert_matches!(
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Singles)
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_2("player-2", "Player 2")
            .add_player_to_side_1("player-3", "Player 3")
            .build(&data)
            .err(),
        Some(err) => {
            assert!(err.full_description().contains("Side 1 has too many players for a singles battle"), "{err:?}");
        }
    );
}

#[test]
fn validates_players_per_side_clause() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    assert_matches::assert_matches!(
        TestBattleBuilder::new()
            .with_battle_type(BattleType::Multi)
            .with_rule("Players Per Side = 2")
            .add_player_to_side_1("player-1", "Player 1")
            .add_player_to_side_1("player-2", "Player 2")
            .add_player_to_side_2("player-3", "Player 3")
            .build(&data)
            .err(),
        Some(err) => {
            assert!(err.full_description().contains("Side 2 must have exactly 2 players."), "{err:?}");
        }
    );
}

#[test]
fn validates_empty_teams_before_start() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle_builder().build(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Err(err) => {
        assert_matches::assert_matches!(err.as_ref().downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Validation failed for Player 1: Empty team is not allowed."), "{err:?}");
            assert!(err.problems().contains(&"Validation failed for Player 2: Empty team is not allowed."), "{err:?}");
        });
    });

    assert_matches::assert_matches!(
        battle.update_team("player-1", three_starters().unwrap()),
        Ok(())
    );
    assert_matches::assert_matches!(battle.start(), Err(err) => {
        assert_matches::assert_matches!(err.as_ref().downcast_ref::<ValidationError>(), Some(err) => {
            assert!(!err.problems().contains(&"Validation failed for Player 1: Empty team is not allowed."), "{err:?}");
            assert!(err.problems().contains(&"Validation failed for Player 2: Empty team is not allowed."), "{err:?}");
        });
    });

    assert_matches::assert_matches!(
        battle.update_team("player-2", three_starters().unwrap()),
        Ok(())
    );
    assert_matches::assert_matches!(battle.start(), Ok(()));
}

#[test]
fn validates_team_legality_during_update() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle_builder()
        .with_rule("Max Team Size = 2")
        .build(&data)
        .unwrap();

    assert_matches::assert_matches!(
        battle.update_team("player-1", three_starters().unwrap()),
        Err(err) => {
            assert_matches::assert_matches!(err.as_ref().downcast_ref::<ValidationError>(), Some(err) => {
                assert!(err.problems().contains(&"You may only bring up to 2 Mons (your team has 3)."), "{err:?}");
            });
        }
    );

    // Team is not saved.
    assert_matches::assert_matches!(battle.start(), Err(err) => {
        assert_matches::assert_matches!(err.as_ref().downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Validation failed for Player 1: Empty team is not allowed."), "{err:?}");
        });
    });
}

#[test]
fn fails_to_update_team_for_nonexistent_player() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle_builder().build(&data).unwrap();

    assert_matches::assert_matches!(
        battle.update_team("player-3", three_starters().unwrap()),
        Err(err) => {
            assert!(err.full_description().contains("not found"), "{err:?}");
        }
    );
}
