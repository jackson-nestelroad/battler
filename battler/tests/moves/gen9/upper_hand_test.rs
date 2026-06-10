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
                    "name": "Okidogi",
                    "species": "Okidogi",
                    "ability": "No Ability",
                    "moves": [
                        "Upper Hand",
                        "Tackle"
                    ],
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
fn upper_hand_flinches_target_when_using_priority_move() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Okidogi,player-1,1|name:Upper Hand|noanim",
            "fail|mon:Okidogi,player-1,1",
            "move|mon:Okidogi,player-2,1|name:Tackle|target:Okidogi,player-1,1",
            "split|side:0",
            "damage|mon:Okidogi,player-1,1|health:249/286",
            "damage|mon:Okidogi,player-1,1|health:88/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Okidogi,player-1,1|name:Upper Hand|target:Okidogi,player-2,1",
            "resisted|mon:Okidogi,player-2,1",
            "split|side:1",
            "damage|mon:Okidogi,player-2,1|health:245/286",
            "damage|mon:Okidogi,player-2,1|health:86/100",
            "cant|mon:Okidogi,player-2,1|from:Flinch",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
