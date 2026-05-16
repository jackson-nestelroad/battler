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
                    "name": "Blacephalon",
                    "species": "Blacephalon",
                    "ability": "No Ability",
                    "moves": [
                        "Mind Blown",
                        "Protect"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Stakataka",
                    "species": "Stakataka",
                    "ability": "No Ability",
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
fn mind_blown_deals_recoil_based_on_user_hp() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blacephalon,player-1,1|name:Mind Blown",
            "resisted|mon:Blacephalon,player-2,1",
            "split|side:1",
            "damage|mon:Blacephalon,player-2,1|health:43/216",
            "damage|mon:Blacephalon,player-2,1|health:20/100",
            "split|side:0",
            "damage|mon:Blacephalon,player-1,1|from:Recoil|health:108/216",
            "damage|mon:Blacephalon,player-1,1|from:Recoil|health:50/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mind_blown_deals_recoil_even_if_hits_protect() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blacephalon,player-2,1|name:Protect|target:Blacephalon,player-2,1",
            "singleturn|mon:Blacephalon,player-2,1|move:Protect",
            "move|mon:Blacephalon,player-1,1|name:Mind Blown|noanim",
            "activate|mon:Blacephalon,player-2,1|move:Protect",
            "split|side:0",
            "damage|mon:Blacephalon,player-1,1|from:Recoil|health:108/216",
            "damage|mon:Blacephalon,player-1,1|from:Recoil|health:50/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mind_blown_does_not_deal_recoil_if_canceled_by_damp() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Damp".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blacephalon,player-1,1|name:Mind Blown|noanim",
            "cant|mon:Blacephalon,player-1,1|from:ability:Damp|of:Blacephalon,player-2,1",
            "fail|mon:Blacephalon,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mind_blown_recoil_activates_emergency_exit() {
    let mut team_1 = team().unwrap();
    team_1.members[0].ability = "Emergency Exit".to_owned();
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blacephalon,player-1,1|name:Mind Blown",
            "resisted|mon:Blacephalon,player-2,1",
            "split|side:1",
            "damage|mon:Blacephalon,player-2,1|health:43/216",
            "damage|mon:Blacephalon,player-2,1|health:20/100",
            "split|side:0",
            "damage|mon:Blacephalon,player-1,1|from:Recoil|health:108/216",
            "damage|mon:Blacephalon,player-1,1|from:Recoil|health:50/100",
            "activate|mon:Blacephalon,player-1,1|ability:Emergency Exit",
            "switchout|mon:Blacephalon,player-1,1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
