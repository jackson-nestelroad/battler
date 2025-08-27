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
    assert_logs_since_start_eq,
};

fn stantler_tauros() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Stantler",
                    "species": "Stantler",
                    "ability": "Frisk",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Tauros",
                    "species": "Tauros",
                    "ability": "No Ability",
                    "moves": [],
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
        .with_battle_type(BattleType::Doubles)
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
fn frisk_announces_foe_items() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = stantler_tauros().unwrap();
    team.members[0].item = Some("Cheri Berry".to_owned());
    team.members[1].item = Some("Life Orb".to_owned());
    let mut battle = make_battle(&data, 0, team, stantler_tauros().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "item|mon:Stantler,player-1,1|item:Cheri Berry|from:ability:Frisk|of:Stantler,player-2,1",
            "item|mon:Tauros,player-1,2|item:Life Orb|from:ability:Frisk|of:Stantler,player-2,1",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
