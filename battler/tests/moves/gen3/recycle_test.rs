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

fn snorlax() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "ability": "No Ability",
                    "moves": [
                        "Recycle",
                        "Will-O-Wisp"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Rawst Berry"
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
fn recycle_recovers_consumed_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, snorlax().unwrap(), snorlax().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snorlax,player-1,1|name:Recycle|noanim",
            "fail|mon:Snorlax,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Snorlax,player-2,1|name:Will-O-Wisp|target:Snorlax,player-1,1",
            "status|mon:Snorlax,player-1,1|status:Burn",
            "itemend|mon:Snorlax,player-1,1|item:Rawst Berry|eat",
            "curestatus|mon:Snorlax,player-1,1|status:Burn|from:item:Rawst Berry",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Snorlax,player-1,1|name:Recycle|target:Snorlax,player-1,1",
            "item|mon:Snorlax,player-1,1|item:Rawst Berry|from:move:Recycle",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Snorlax,player-2,1|name:Will-O-Wisp|target:Snorlax,player-1,1",
            "status|mon:Snorlax,player-1,1|status:Burn",
            "itemend|mon:Snorlax,player-1,1|item:Rawst Berry|eat",
            "curestatus|mon:Snorlax,player-1,1|status:Burn|from:item:Rawst Berry",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
