use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::TestBattleBuilder;

fn singles_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Torrent",
                    "moves": [
                        "Blizzard",
                        "Counter",
                        "Hail",
                        "Scald"
                    ],
                    "nature": "Adamant",
                    "gender": "M",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_singles_battle(data: &dyn DataStore) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_auto_continue(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", singles_team()?)
        .with_team("player-2", singles_team()?)
        .build(data)
}

fn singles_team_no_moves() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Torrent",
                    "moves": [],
                    "nature": "Adamant",
                    "gender": "M",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_singles_battle_with_struggle(data: &dyn DataStore) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_auto_continue(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", singles_team_no_moves()?)
        .with_team("player-2", singles_team_no_moves()?)
        .build(data)
}

fn player_request(battle: &PublicCoreBattle, player_id: &str) -> Option<Request> {
    battle
        .active_requests()
        .find(|(player, _)| player == player_id)
        .map(|(_, request)| request)
}

fn player_has_active_request(battle: &PublicCoreBattle, player_id: &str) -> bool {
    player_request(battle, player_id).is_some()
}

#[test]
fn too_many_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0; move 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 1: cannot move: you sent more choices than active mons")
    );
    assert!(player_has_active_request(&battle, "player-1"));
}

#[test]
fn missing_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: missing move choice")
    );
    assert!(player_has_active_request(&battle, "player-1"));
}

#[test]
fn invalid_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 5"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Blastoise does not have a move in slot 5")
    );
    assert!(player_has_active_request(&battle, "player-1"));
}

#[test]
fn target_not_allowed() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0, 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: you cannot choose a target for Blizzard")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1, 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: you cannot choose a target for Counter")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2, 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: you cannot choose a target for Hail")
    );
    assert!(player_has_active_request(&battle, "player-1"));
}

#[test]
fn target_implied_in_singles() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert!(!player_has_active_request(&battle, "player-1"));
}

#[test]
fn target_chosen_for_singles() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3, 1"), Ok(()));
    assert!(!player_has_active_request(&battle, "player-1"));
}

#[test]
fn target_out_of_bounds() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3, 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Scald")
    );
    assert!(player_has_active_request(&battle, "player-1"));
}

#[test]
fn struggle_when_no_available_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_singles_battle_with_struggle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    assert!(
        player_request(&battle, "player-1").is_some_and(|request| match request {
            Request::Turn(request) => request.active.first().is_some_and(|mon| mon.moves.len()
                == 1
                && mon.moves.first().is_some_and(
                    |move_slot| move_slot.name == "Struggle" && move_slot.id.eq("struggle")
                )),
            _ => false,
        })
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert!(!player_has_active_request(&battle, "player-1"));
}

fn triples_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mew",
                    "species": "Mew",
                    "ability": "Synchronize",
                    "moves": [
                        "Helping Hand",
                        "Giga Drain",
                        "Outrage",
                        "Me First"
                    ],
                    "nature": "Adamant",
                    "gender": "U",
                    "level": 100
                },
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "Blaze",
                    "moves": [
                        "Aerial Ace",
                        "Blast Burn",
                        "Air Cutter",
                        "Heat Wave"
                    ],
                    "nature": "Adamant",
                    "gender": "M",
                    "level": 100
                },
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Torrent",
                    "moves": [
                        "Surf",
                        "Water Pulse",
                        "Hail",
                        "Scald"
                    ],
                    "nature": "Adamant",
                    "gender": "M",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_triples_battle(data: &dyn DataStore) -> Result<PublicCoreBattle> {
    // Adjacency rules really only matter for Triples, so we use a Triples battle to verify our
    // adjacency rules.
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Triples)
        .with_auto_continue(false)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", triples_team()?)
        .with_team("player-2", triples_team()?)
        .build(data)
}

#[test]
fn target_normal() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_triples_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    // All three Mons choose a move that must target an adjacent Mon.

    // Target foes.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1; move 1,1; move 3,3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Giga Drain")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2; move 1,1; move 3,3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 2: cannot move: invalid target for Scald")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,3; move 1,2; move 3,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,3; move 1,3; move 3,1"),
        Ok(())
    );

    // Target allies.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,-1; move 1,1; move 3,-3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Giga Drain")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,-3; move 1,1; move 3,-3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Giga Drain")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,-2; move 1,-2; move 3,-3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 1: cannot move: invalid target for Blast Burn")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,-2; move 1,-1; move 3,-3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 2: cannot move: invalid target for Scald")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,-2; move 1,-3; move 3,-1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 2: cannot move: invalid target for Scald")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,-2; move 1,-3; move 3,-2"),
        Ok(())
    );

    assert!(!player_has_active_request(&battle, "player-1"));
}

#[test]
fn target_any_except_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_triples_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));
    // Blastoise's Water Pulse can hit a non-adjacent foe.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2; move 2; move 1,3"),
        Ok(())
    );
    // But it cannot hit itself.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 2; move 2; move 1,-3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 2: cannot move: invalid target for Water Pulse")
    );
}

#[test]
fn target_adjacent_foe() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_triples_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    // Adjacent ally or self is not allowed.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,-2; move 2; move 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Me First")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,-1; move 2; move 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Me First")
    );

    // Adjacent foe is allowed.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,1; move 2; move 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Me First")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,2; move 2; move 2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,3; move 2; move 2"),
        Ok(())
    );
}

#[test]
fn target_adjacent_ally() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_triples_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    // Adjacent foe or self is not allowed.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,3; move 2; move 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Helping Hand")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2; move 2; move 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Helping Hand")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-1; move 2; move 2"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Helping Hand")
    );

    // Adjacent ally is allowed.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2; move 2; move 2"),
        Ok(())
    );
}

#[test]
fn target_adjacent_ally_or_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let _ = make_triples_battle(&data).unwrap();
    // TODO: Use an AdjacentAllyOrUser move.
    // Acupressure is the only move that does this.
}

fn make_multi_battle(data: &dyn DataStore) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Multi)
        .with_auto_continue(false)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_1("player-2", "Player 2")
        .add_player_to_side_1("player-3", "Player 3")
        .add_player_to_side_2("player-4", "Player 4")
        .add_player_to_side_2("player-5", "Player 5")
        .add_player_to_side_2("player-6", "Player 6")
        .with_team("player-1", singles_team()?)
        .with_team("player-2", singles_team()?)
        .with_team("player-3", singles_team()?)
        .with_team("player-4", singles_team()?)
        .with_team("player-5", singles_team()?)
        .with_team("player-6", singles_team()?)
        .build(data)
}

#[test]
fn adjacency_rules_apply_across_players() {
    // Use a 3v3 Multi battle to determine if the adjacnency rules behave the same as a Triples
    // battle.
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_multi_battle(&data).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.continue_battle(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 3,1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Scald")
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3,2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3,3"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3,2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3,3"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-3", "move 3,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "move 3,2"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-3", "move 3,3"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: invalid target for Scald")
    );
}
