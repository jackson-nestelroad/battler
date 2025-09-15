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

fn drifblim() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Drifblim",
                    "species": "Drifblim",
                    "ability": "Aftermath",
                    "moves": [
                        "Thunder Punch"
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
fn aftermath_deals_damage_to_opponent_on_contact_on_faint() {
    let mut battle = make_battle(0, drifblim().unwrap(), drifblim().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Drifblim,player-2,1|name:Thunder Punch|target:Drifblim,player-1,1",
            "supereffective|mon:Drifblim,player-1,1",
            "split|side:0",
            "damage|mon:Drifblim,player-1,1|health:0",
            "damage|mon:Drifblim,player-1,1|health:0",
            "split|side:1",
            "damage|mon:Drifblim,player-2,1|from:ability:Aftermath|of:Drifblim,player-1,1|health:158/210",
            "damage|mon:Drifblim,player-2,1|from:ability:Aftermath|of:Drifblim,player-1,1|health:76/100",
            "faint|mon:Drifblim,player-1,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
