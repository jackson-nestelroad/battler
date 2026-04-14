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
                    "name": "Florges",
                    "species": "Florges",
                    "ability": "Symbiosis",
                    "item": "Cheri Berry",
                    "moves": [
                        "Toxic",
                        "Thunder Wave"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Florges",
                    "species": "Florges",
                    "ability": "No Ability",
                    "item": "Lum Berry",
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
        .with_battle_type(BattleType::Doubles)
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
fn symbiosis_passes_item_to_ally_after_using_item() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Florges,player-1,1|name:Toxic|target:Florges,player-2,2",
            "status|mon:Florges,player-2,2|status:Bad Poison",
            "itemend|mon:Florges,player-2,2|item:Lum Berry|eat",
            "curestatus|mon:Florges,player-2,2|status:Bad Poison|from:item:Lum Berry",
            "itemend|mon:Florges,player-2,1|item:Cheri Berry|from:ability:Symbiosis",
            "item|mon:Florges,player-2,2|item:Cheri Berry|from:ability:Symbiosis|of:Florges,player-2,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Florges,player-1,1|name:Thunder Wave|target:Florges,player-2,2",
            "status|mon:Florges,player-2,2|status:Paralysis",
            "itemend|mon:Florges,player-2,2|item:Cheri Berry|eat",
            "curestatus|mon:Florges,player-2,2|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
