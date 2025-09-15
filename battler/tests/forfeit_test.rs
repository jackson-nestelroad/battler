use std::time::Duration;

use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

fn make_multi_battle(
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
    team_3: TeamData,
    team_4: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_1("player-2", "Player 2")
        .add_player_to_side_2("player-3", "Player 3")
        .add_player_to_side_2("player-4", "Player 4")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .with_team("player-3", team_3)
        .with_team("player-4", team_4)
        .build(static_local_data_store())
}

#[test]
fn forfeit_ends_singles_battle() {
    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "forfeit"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "switchout|mon:Bulbasaur,player-1,1",
            "forfeited|player:player-1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forfeit_ends_doubles_battle() {
    let mut battle = make_battle(BattleType::Doubles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;forfeit"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "switchout|mon:Bulbasaur,player-1,1",
            "switchout|mon:Charmander,player-1,2",
            "forfeited|player:player-1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forfeit_ends_multi_battle() {
    let mut battle = make_multi_battle(
        BattleType::Multi,
        0,
        team().unwrap(),
        team().unwrap(),
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "forfeit"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-4", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: you left the battle")
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "forfeit"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-4", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "switchout|mon:Bulbasaur,player-1,1",
            "forfeited|player:player-1",
            "move|mon:Bulbasaur,player-2,2|name:Tackle|target:Bulbasaur,player-4,2",
            "split|side:1",
            ["damage", "Bulbasaur,player-4,2"],
            ["damage", "Bulbasaur,player-4,2"],
            "move|mon:Bulbasaur,player-3,1|name:Tackle|target:Bulbasaur,player-2,2",
            "split|side:0",
            ["damage", "Bulbasaur,player-2,2"],
            ["damage", "Bulbasaur,player-2,2"],
            "move|mon:Bulbasaur,player-4,2|name:Tackle|target:Bulbasaur,player-2,2",
            "split|side:0",
            ["damage", "Bulbasaur,player-2,2"],
            ["damage", "Bulbasaur,player-2,2"],
            "residual",
            "turn|turn:2",
            ["time"],
            "switchout|mon:Bulbasaur,player-2,2",
            "forfeited|player:player-2",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn forfeit_order_determined_by_time() {
    let mut battle = make_multi_battle(
        BattleType::Multi,
        0,
        team().unwrap(),
        team().unwrap(),
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "forfeit"), Ok(()));
    std::thread::sleep(Duration::from_secs(1));
    assert_matches::assert_matches!(battle.set_player_choice("player-3", "forfeit"), Ok(()));
    std::thread::sleep(Duration::from_secs(1));
    assert_matches::assert_matches!(battle.set_player_choice("player-4", "forfeit"), Ok(()));
    std::thread::sleep(Duration::from_secs(1));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "forfeit"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "switchout|mon:Bulbasaur,player-1,1",
            "forfeited|player:player-1",
            "switchout|mon:Bulbasaur,player-3,1",
            "forfeited|player:player-3",
            "switchout|mon:Bulbasaur,player-4,2",
            "forfeited|player:player-4",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
