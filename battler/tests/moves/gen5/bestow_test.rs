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
    assert_turn_logs_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pidove",
                    "species": "Pidove",
                    "ability": "No Ability",
                    "moves": [
                        "Bestow",
                        "Fling"
                    ],
                    "nature": "Hardy",
                    "gender": "F",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_seed(seed)
        .with_battle_type(BattleType::Singles)
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
fn bestow_transfers_held_item() {
    let mut team_1 = team().unwrap();
    team_1.members[0].item = Some("Sticky Barb".to_owned());
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pidove,player-1,1|name:Bestow|target:Pidove,player-2,1",
            "itemend|mon:Pidove,player-1,1|item:Sticky Barb|from:move:Bestow",
            "item|mon:Pidove,player-2,1|item:Sticky Barb|from:move:Bestow|of:Pidove,player-1,1",
            "split|side:1",
            "damage|mon:Pidove,player-2,1|from:item:Sticky Barb|health:184/210",
            "damage|mon:Pidove,player-2,1|from:item:Sticky Barb|health:88/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 1, &expected_logs);
}

#[test]
fn bestow_fails_if_target_has_item() {
    let mut team = team().unwrap();
    team.members[0].item = Some("Sticky Barb".to_owned());
    let mut battle = make_battle(0, team.clone(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pidove,player-1,1|name:Bestow|noanim",
            "itemend|mon:Pidove,player-1,1|item:Sticky Barb|from:move:Bestow",
            "fail|mon:Pidove,player-1,1",
            "split|side:1",
            "damage|mon:Pidove,player-2,1|from:item:Sticky Barb|health:184/210",
            "damage|mon:Pidove,player-2,1|from:item:Sticky Barb|health:88/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 1, &expected_logs);
}
