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

fn mantyke() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mantyke",
                    "species": "Mantyke",
                    "ability": "No Ability",
                    "moves": [
                        "Aqua Ring",
                        "Slash"
                    ],
                    "nature": "Hardy",
                    "level": 50
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
fn aqua_ring_heals_each_turn() {
    let mut battle = make_battle(0, mantyke().unwrap(), mantyke().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mantyke,player-1,1|name:Aqua Ring|target:Mantyke,player-1,1",
            "start|mon:Mantyke,player-1,1|move:Aqua Ring",
            "move|mon:Mantyke,player-2,1|name:Slash|target:Mantyke,player-1,1",
            "split|side:0",
            "damage|mon:Mantyke,player-1,1|health:90/105",
            "damage|mon:Mantyke,player-1,1|health:86/100",
            "split|side:0",
            "heal|mon:Mantyke,player-1,1|from:move:Aqua Ring|health:96/105",
            "heal|mon:Mantyke,player-1,1|from:move:Aqua Ring|health:92/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
