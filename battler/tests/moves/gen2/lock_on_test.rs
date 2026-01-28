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

fn nosepass() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Nosepass",
                    "species": "Nosepass",
                    "ability": "No Ability",
                    "moves": [
                        "Lock On",
                        "Dig",
                        "Zap Cannon"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Reverse)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn lock_on_removes_accuracy_and_invulnerability() {
    let mut battle = make_battle(998989898, nosepass().unwrap(), nosepass().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Nosepass,player-2,1|name:Lock-On|target:Nosepass,player-1,1",
            "activate|mon:Nosepass,player-2,1|move:Lock-On|of:Nosepass,player-1,1",
            "move|mon:Nosepass,player-1,1|name:Dig|noanim",
            "prepare|mon:Nosepass,player-1,1|move:Dig",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Nosepass,player-2,1|name:Zap Cannon|target:Nosepass,player-1,1",
            "split|side:0",
            "damage|mon:Nosepass,player-1,1|health:64/90",
            "damage|mon:Nosepass,player-1,1|health:72/100",
            "status|mon:Nosepass,player-1,1|status:Paralysis",
            "move|mon:Nosepass,player-1,1|name:Dig|target:Nosepass,player-2,1",
            "supereffective|mon:Nosepass,player-2,1",
            "split|side:1",
            "damage|mon:Nosepass,player-2,1|health:64/90",
            "damage|mon:Nosepass,player-2,1|health:72/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
