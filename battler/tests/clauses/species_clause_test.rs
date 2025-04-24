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
fn enforces_unique_species() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle_builder()
        .with_rule("Species Clause")
        .build(&data)
        .unwrap();

    let mut bad_team = three_starters().unwrap();
    bad_team.members[1].species = "Bulbasaur".to_owned();

    assert_matches::assert_matches!(battle.update_team("player-1", bad_team), Ok(()));

    assert_matches::assert_matches!(battle.validate_player("player-1"), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Species Bulbasaur appears more than 1 time."), "{err:?}");
        });
    });
    assert_matches::assert_matches!(battle.start(), Err(err) => {
        assert_matches::assert_matches!(err.downcast_ref::<ValidationError>(), Some(err) => {
            assert!(err.problems().contains(&"Validation failed for Player 1: Species Bulbasaur appears more than 1 time."), "{err:?}");
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
