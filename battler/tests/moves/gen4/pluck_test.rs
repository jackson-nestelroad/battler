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

fn starly() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Starly",
                    "species": "Starly",
                    "ability": "No Ability",
                    "moves": [
                        "Pluck",
                        "Toxic"
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
fn pluck_does_not_consume_non_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = starly().unwrap();
    team.members[0].item = Some("Leftovers".to_owned());
    let mut battle = make_battle(&data, 0, starly().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Starly,player-1,1|name:Pluck|target:Starly,player-2,1",
            "split|side:1",
            "damage|mon:Starly,player-2,1|health:33/100",
            "damage|mon:Starly,player-2,1|health:33/100",
            "split|side:1",
            "heal|mon:Starly,player-2,1|from:item:Leftovers|health:39/100",
            "heal|mon:Starly,player-2,1|from:item:Leftovers|health:39/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn pluck_consumes_berry_with_no_effect() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = starly().unwrap();
    team.members[0].item = Some("Pecha Berry".to_owned());
    let mut battle = make_battle(&data, 0, starly().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Starly,player-1,1|name:Pluck|target:Starly,player-2,1",
            "split|side:1",
            "damage|mon:Starly,player-2,1|health:33/100",
            "damage|mon:Starly,player-2,1|health:33/100",
            "itemend|mon:Starly,player-2,1|item:Pecha Berry|from:move:Pluck|of:Starly,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn pluck_consumes_berry_with_effect() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = starly().unwrap();
    team.members[0].item = Some("Pecha Berry".to_owned());
    let mut battle = make_battle(&data, 0, starly().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Starly,player-1,1|name:Pluck|target:Starly,player-2,1",
            "split|side:1",
            "damage|mon:Starly,player-2,1|health:33/100",
            "damage|mon:Starly,player-2,1|health:33/100",
            "itemend|mon:Starly,player-2,1|item:Pecha Berry|from:move:Pluck|of:Starly,player-1,1",
            "curestatus|mon:Starly,player-1,1|status:Bad Poison|from:item:Pecha Berry",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
