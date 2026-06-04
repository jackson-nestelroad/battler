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
    assert_logs_since_start_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Xerneas",
                    "species": "Xerneas-Neutral",
                    "ability": "Fairy Aura",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn xerneas_transforms_out_of_neutral_forme_on_start() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "species:Xerneas-Neutral"],
            ["switch", "player-1", "species:Xerneas-Neutral"],
            "split|side:1",
            ["switch", "player-2", "species:Xerneas-Neutral"],
            ["switch", "player-2", "species:Xerneas-Neutral"],
            "split|side:0",
            ["specieschange", "player-1", "species:Xerneas|"],
            ["specieschange", "player-1", "species:Xerneas|"],
            "formechange|mon:Xerneas,player-1,1|species:Xerneas|from:species:Xerneas-Neutral",
            "ability|mon:Xerneas,player-1,1|ability:Fairy Aura",
            "split|side:1",
            ["specieschange", "player-2", "species:Xerneas|"],
            ["specieschange", "player-2", "species:Xerneas|"],
            "formechange|mon:Xerneas,player-2,1|species:Xerneas|from:species:Xerneas-Neutral",
            "ability|mon:Xerneas,player-2,1|ability:Fairy Aura",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
