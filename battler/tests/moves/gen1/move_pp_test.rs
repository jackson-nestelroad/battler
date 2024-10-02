use assert_matches::assert_matches;
use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
        Request,
    },
    common::{
        Error,
        Id,
        WrapResultError,
    },
    dex::DataStore,
    moves::MoveData,
    teams::TeamData,
};
use battler_test_utils::{
    assert_error_message,
    TestBattleBuilder,
    TestDataStore,
};

fn team(pp_boosts: Vec<u8>) -> Result<TeamData, Error> {
    let mut team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Test Move 1",
                        "Test Move 2",
                        "Test Move 3",
                        "Test Move 4"
                    ],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()?;
    team.members[0].pp_boosts = pp_boosts;
    Ok(team)
}

fn test_move(name: &str, pp: u8) -> Result<MoveData, Error> {
    let mut move_data: MoveData = serde_json::from_str(
        r#"{
            "name": "",
            "category": "Physical",
            "primary_type": "Normal",
            "base_power": 1,
            "accuracy": "exempt",
            "pp": 0,
            "target": "Normal",
            "flags": []
        }"#,
    )
    .wrap_error()?;
    move_data.name = name.to_owned();
    move_data.pp = pp;
    Ok(move_data)
}

fn add_test_moves(data: &mut TestDataStore) -> Result<(), Error> {
    data.add_fake_move(Id::from("Test Move 1"), test_move("Test Move 1", 5)?);
    data.add_fake_move(Id::from("Test Move 2"), test_move("Test Move 2", 10)?);
    data.add_fake_move(Id::from("Test Move 3"), test_move("Test Move 3", 35)?);
    data.add_fake_move(Id::from("Test Move 4"), test_move("Test Move 4", 40)?);
    Ok(())
}

fn make_battle(data: &dyn DataStore, pp_boosts: Vec<u8>) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_actual_health(true)
        .with_pass_allowed(true)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("test-player", "Test Player")
        .add_player_to_side_2("foe", "Foe")
        .with_team("test-player", team(pp_boosts.clone())?)
        .with_team("foe", team(pp_boosts)?)
        .build(data)
}

#[test]
fn using_move_reduces_pp() {
    let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
    add_test_moves(&mut data).unwrap();
    let mut battle = make_battle(&data, Vec::new()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {
        assert_eq!(
            request.active[0]
                .moves
                .iter()
                .map(|mov| mov.pp)
                .collect::<Vec<_>>(),
            vec![5, 10, 35, 40]
        );
    });

    assert_eq!(battle.set_player_choice("foe", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("test-player", "move 0"), Ok(()));

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {
        assert_eq!(
            request.active[0]
                .moves
                .iter()
                .map(|mov| mov.pp)
                .collect::<Vec<_>>(),
            vec![4, 10, 35, 40]
        );
    });

    assert_eq!(battle.set_player_choice("foe", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("test-player", "move 0"), Ok(()));

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {assert_eq!(
        request.active[0]
            .moves
            .iter()
            .map(|mov| mov.pp)
            .collect::<Vec<_>>(),
        vec![3, 10, 35, 40]
    );
    });

    assert_eq!(battle.set_player_choice("foe", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("test-player", "move 1"), Ok(()));

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {assert_eq!(
            request.active[0]
                .moves
                .iter()
                .map(|mov| mov.pp)
                .collect::<Vec<_>>(),
            vec![3, 9, 35, 40]
        );
    });
}

#[test]
fn pp_boosts_increase_pp() {
    let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
    add_test_moves(&mut data).unwrap();
    let mut battle = make_battle(&data, vec![1, 1, 1, 1]).unwrap();
    assert_eq!(battle.start(), Ok(()));

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {
        assert_eq!(
            request.active[0]
                .moves
                .iter()
                .map(|mov| mov.pp)
                .collect::<Vec<_>>(),
            vec![6, 12, 42, 48]
        );
    });

    let mut battle = make_battle(&data, vec![2, 2, 2, 2]).unwrap();
    assert_eq!(battle.start(), Ok(()));

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {
        assert_eq!(
            request.active[0]
                .moves
                .iter()
                .map(|mov| mov.pp)
                .collect::<Vec<_>>(),
            vec![7, 14, 49, 56]
        );
    });

    let mut battle = make_battle(&data, vec![3, 3, 3, 3]).unwrap();
    assert_eq!(battle.start(), Ok(()));

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {
        assert_eq!(
            request.active[0]
                .moves
                .iter()
                .map(|mov| mov.pp)
                .collect::<Vec<_>>(),
            vec![8, 16, 56, 64]
        );
    });

    // PP boosts max out at 3.
    let mut battle = make_battle(&data, vec![4, 4, 4, 4]).unwrap();
    assert_eq!(battle.start(), Ok(()));

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {
        assert_eq!(
            request.active[0]
                .moves
                .iter()
                .map(|mov| mov.pp)
                .collect::<Vec<_>>(),
            vec![8, 16, 56, 64]
        );
    });
}

#[test]
fn move_runs_out_of_pp() {
    let mut data = TestDataStore::new_from_env("DATA_DIR").unwrap();
    add_test_moves(&mut data).unwrap();
    let mut battle = make_battle(&data, Vec::new()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    // Use all the PP for the first move.
    for _ in 0..5 {
        assert_eq!(battle.set_player_choice("foe", "pass"), Ok(()));
        assert_eq!(battle.set_player_choice("test-player", "move 0"), Ok(()));
    }

    let request = battle.request_for_player("test-player");
    assert_matches!(request, Some(Request::Turn(request)) => {
        assert_eq!(
            request.active[0]
                .moves
                .iter()
                .map(|mov| mov.pp)
                .collect::<Vec<_>>(),
            vec![0, 10, 35, 40]
        );
    });

    assert_error_message(
        battle.set_player_choice("test-player", "move 0"),
        "cannot move: Venusaur's Test Move 1 is disabled",
    );
    assert_eq!(battle.ready_to_continue(), Ok(false));
}
