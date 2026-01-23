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
                    "name": "Archeops",
                    "species": "Archeops",
                    "ability": "No Ability",
                    "moves": [
                        "Acrobatics",
                        "Knock Off"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "item": "Oran Berry"
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
        .with_base_damage_randomization(battler::CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn acrobatics_doubles_power_when_user_has_no_item() {
    let team_1 = team().unwrap();
    let team_2 = team().unwrap();
    let mut battle = make_battle(0, team_1, team_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Acrobatics with item.
    // Player 2 uses Knock Off to remove Player 1's item.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    // Turn 2: Acrobatics without item.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Archeops,player-1,1|name:Acrobatics|target:Archeops,player-2,1",
            "resisted|mon:Archeops,player-2,1",
            "split|side:1",
            "damage|mon:Archeops,player-2,1|health:186/260",
            "damage|mon:Archeops,player-2,1|health:72/100",
            "move|mon:Archeops,player-2,1|name:Knock Off|target:Archeops,player-1,1",
            "split|side:0",
            "damage|mon:Archeops,player-1,1|health:86/260",
            "damage|mon:Archeops,player-1,1|health:34/100",
            "itemend|mon:Archeops,player-1,1|item:Oran Berry|from:move:Knock Off|of:Archeops,player-2,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Archeops,player-1,1|name:Acrobatics|target:Archeops,player-2,1",
            "resisted|mon:Archeops,player-2,1",
            "split|side:1",
            "damage|mon:Archeops,player-2,1|health:39/260",
            "damage|mon:Archeops,player-2,1|health:15/100",
            "itemend|mon:Archeops,player-2,1|item:Oran Berry|eat",
            "split|side:1",
            "heal|mon:Archeops,player-2,1|from:item:Oran Berry|health:49/260",
            "heal|mon:Archeops,player-2,1|from:item:Oran Berry|health:19/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
