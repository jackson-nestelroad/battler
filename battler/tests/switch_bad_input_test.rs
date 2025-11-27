use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    TestBattleBuilder,
    static_local_data_store,
};

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

fn make_battle() -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team()?)
        .with_team("player-2", team()?)
        .build(static_local_data_store())
}

#[test]
fn too_many_switches() {
    let mut battle = make_battle().unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2; switch 3; switch 4"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 2: cannot switch: you sent more choices than active mons")
    );
}

#[test]
fn missing_position() {
    let mut battle = make_battle().unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch;switch"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch  ;switch "),
        Ok(())
    );
}

#[test]
fn invalid_position() {
    let mut battle = make_battle().unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch Charmander"),
        Err(err) => assert!(format!("{err:#}").contains("invalid choice 0: invalid digit"), "{err:#}")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch -1"),
        Err(err) => assert!(format!("{err:#}").contains("invalid choice 0: invalid digit"), "{err:#}")
    );
}

#[test]
fn no_mon_in_position() {
    let mut battle = make_battle().unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 6"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 6 to switch to")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 10"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you do not have a mon in slot 10 to switch to")
    );
}

#[test]
fn switch_to_active_mon() {
    let mut battle = make_battle().unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you cannot switch to an active mon")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: you cannot switch to an active mon")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2; switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 1: cannot switch: you cannot switch to an active mon")
    );
}

#[test]
fn switch_a_mon_in_twice() {
    let mut battle = make_battle().unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2; switch 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 1: cannot switch: the mon in slot 2 can only switch in once")
    );
}
