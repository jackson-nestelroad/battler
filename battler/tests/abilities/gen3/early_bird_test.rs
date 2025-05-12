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
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn seedot() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Seedot",
                    "species": "Seedot",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Rest"
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
) -> Result<PublicCoreBattle> {
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
fn early_bird_wakes_up_twice_as_fast() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = seedot().unwrap();
    team.members[0].ability = "Early Bird".to_owned();
    let mut battle = make_battle(&data, 0, team, seedot().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Seedot,player-1,1|name:Rest|target:Seedot,player-1,1",
            "status|mon:Seedot,player-1,1|status:Sleep",
            "split|side:0",
            "heal|mon:Seedot,player-1,1|health:100/100",
            "heal|mon:Seedot,player-1,1|health:100/100",
            "move|mon:Seedot,player-2,1|name:Rest|target:Seedot,player-2,1",
            "status|mon:Seedot,player-2,1|status:Sleep",
            "split|side:1",
            "heal|mon:Seedot,player-2,1|health:100/100",
            "heal|mon:Seedot,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "cant|mon:Seedot,player-1,1|from:status:Sleep",
            "cant|mon:Seedot,player-2,1|from:status:Sleep",
            "residual",
            "turn|turn:4",
            ["time"],
            "curestatus|mon:Seedot,player-1,1|status:Sleep",
            "move|mon:Seedot,player-1,1|name:Rest|target:Seedot,player-1,1",
            "fail|mon:Seedot,player-1,1|what:heal",
            "cant|mon:Seedot,player-2,1|from:status:Sleep",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Seedot,player-1,1|name:Rest|target:Seedot,player-1,1",
            "fail|mon:Seedot,player-1,1|what:heal",
            "curestatus|mon:Seedot,player-2,1|status:Sleep",
            "move|mon:Seedot,player-2,1|name:Rest|target:Seedot,player-2,1",
            "fail|mon:Seedot,player-2,1|what:heal",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
