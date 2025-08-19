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
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn munchlax() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Munchlax",
                    "species": "Munchlax",
                    "ability": "No Ability",
                    "moves": [
                        "Last Resort",
                        "Growl",
                        "Agility"
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
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn last_resort_fails_if_only_move_known() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = munchlax().unwrap();
    team.members[0].moves.remove(1);
    let mut battle = make_battle(&data, 0, team, munchlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Munchlax,player-1,1|name:Last Resort|noanim",
            "fail|mon:Munchlax,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn last_resort_only_usable_after_all_other_moves_used() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, munchlax().unwrap(), munchlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Munchlax,player-1,1|name:Last Resort|noanim",
            "fail|mon:Munchlax,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Munchlax,player-1,1|name:Growl",
            "unboost|mon:Munchlax,player-2,1|stat:atk|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Munchlax,player-1,1|name:Last Resort|noanim",
            "fail|mon:Munchlax,player-1,1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Munchlax,player-1,1|name:Agility|target:Munchlax,player-1,1",
            "boost|mon:Munchlax,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Munchlax,player-1,1|name:Last Resort|target:Munchlax,player-2,1",
            "split|side:1",
            "damage|mon:Munchlax,player-2,1|health:15/195",
            "damage|mon:Munchlax,player-2,1|health:8/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn last_resort_fails_if_not_known() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = munchlax().unwrap();
    team.members[0].moves = Vec::from_iter(["Copycat".to_owned(), "Agility".to_owned()]);
    let mut battle = make_battle(&data, 0, munchlax().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Munchlax,player-1,1|name:Last Resort|target:Munchlax,player-2,1",
            "split|side:1",
            "damage|mon:Munchlax,player-2,1|health:15/195",
            "damage|mon:Munchlax,player-2,1|health:8/100",
            "move|mon:Munchlax,player-2,1|name:Copycat|target:Munchlax,player-2,1",
            "move|mon:Munchlax,player-2,1|name:Last Resort|from:move:Copycat|noanim",
            "fail|mon:Munchlax,player-2,1",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 5, &expected_logs);
}
