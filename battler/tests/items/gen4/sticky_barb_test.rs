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

fn pachirisu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pachirisu",
                    "species": "Pachirisu",
                    "ability": "Sticky Hold",
                    "item": "Sticky Barb",
                    "moves": [
                        "Tackle"
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
fn sticky_barb_deals_residual_damage_and_transfers_with_contact() {
    let mut team = pachirisu().unwrap();
    team.members[0].item = None;
    let mut battle = make_battle(0, pachirisu().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            "damage|mon:Pachirisu,player-1,1|from:item:Sticky Barb|health:105/120",
            "damage|mon:Pachirisu,player-1,1|from:item:Sticky Barb|health:88/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pachirisu,player-2,1|name:Tackle|target:Pachirisu,player-1,1",
            "split|side:0",
            "damage|mon:Pachirisu,player-1,1|health:93/120",
            "damage|mon:Pachirisu,player-1,1|health:78/100",
            "itemend|mon:Pachirisu,player-1,1|item:Sticky Barb|from:item:Sticky Barb",
            "item|mon:Pachirisu,player-2,1|item:Sticky Barb|from:item:Sticky Barb|of:Pachirisu,player-1,1",
            "split|side:1",
            "damage|mon:Pachirisu,player-2,1|from:item:Sticky Barb|health:105/120",
            "damage|mon:Pachirisu,player-2,1|from:item:Sticky Barb|health:88/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Pachirisu,player-1,1|name:Tackle|target:Pachirisu,player-2,1",
            "split|side:1",
            "damage|mon:Pachirisu,player-2,1|health:94/120",
            "damage|mon:Pachirisu,player-2,1|health:79/100",
            "itemend|mon:Pachirisu,player-2,1|item:Sticky Barb|from:item:Sticky Barb",
            "item|mon:Pachirisu,player-1,1|item:Sticky Barb|from:item:Sticky Barb|of:Pachirisu,player-2,1",
            "move|mon:Pachirisu,player-2,1|name:Tackle|target:Pachirisu,player-1,1",
            "split|side:0",
            "damage|mon:Pachirisu,player-1,1|health:82/120",
            "damage|mon:Pachirisu,player-1,1|health:69/100",
            "itemend|mon:Pachirisu,player-1,1|item:Sticky Barb|from:item:Sticky Barb",
            "item|mon:Pachirisu,player-2,1|item:Sticky Barb|from:item:Sticky Barb|of:Pachirisu,player-1,1",
            "split|side:1",
            "damage|mon:Pachirisu,player-2,1|from:item:Sticky Barb|health:79/120",
            "damage|mon:Pachirisu,player-2,1|from:item:Sticky Barb|health:66/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
