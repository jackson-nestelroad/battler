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

fn abra() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Abra",
                    "species": "Abra",
                    "ability": "Synchronize",
                    "moves": [
                        "Thunder Wave",
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
fn synchronize_synchronizes_status_changes() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = abra().unwrap();
    team.members[0].item = Some("Cheri Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, abra().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Abra,player-1,1|name:Thunder Wave|target:Abra,player-2,1",
            "status|mon:Abra,player-2,1|status:Paralysis",
            "activate|mon:Abra,player-2,1|ability:Synchronize",
            "status|mon:Abra,player-1,1|status:Paralysis|from:ability:Synchronize|of:Abra,player-2,1",
            "activate|mon:Abra,player-1,1|ability:Synchronize",
            "itemend|mon:Abra,player-1,1|item:Cheri Berry|eat",
            "curestatus|mon:Abra,player-1,1|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
