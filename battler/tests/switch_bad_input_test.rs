use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::TestBattleBuilder;

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Chlorophyll",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 50
                },
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Chlorophyll",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 1
                },
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Chlorophyll",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 1
                },
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Chlorophyll",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 1
                },
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Chlorophyll",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 1
                },
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Chlorophyll",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 1
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(data: &dyn DataStore) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team()?)
        .with_team("player-2", team()?)
        .build(data)
}

#[test]
fn too_many_switches() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2; switch 3; switch 4"),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: you sent more choices than active mons")
    );
}

#[test]
fn missing_position() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch"),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: you must select a mon to switch in")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch  "),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: you must select a mon to switch in")
    );
}

#[test]
fn invalid_position() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch Charmander"),
        Err(err) => assert!(format!("{err:#}").contains("cannot switch: switch argument is not an integer"))
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch -1"),
        Err(err) => assert!(format!("{err:#}").contains("cannot switch: switch argument is not an integer"))
    );
}

#[test]
fn no_mon_in_position() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 6"),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: you do not have a mon in slot 6 to switch to")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 10"),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: you do not have a mon in slot 10 to switch to")
    );
}

#[test]
fn switch_to_active_mon() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: you cannot switch to an active mon")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: you cannot switch to an active mon")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2; switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: you cannot switch to an active mon")
    );
}

#[test]
fn switch_a_mon_in_twice() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2; switch 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "cannot switch: the mon in slot 2 can only switch in once")
    );
}
