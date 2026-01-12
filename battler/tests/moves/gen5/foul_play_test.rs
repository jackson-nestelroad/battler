use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn krokorok() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Krokorok",
                    "species": "Krokorok",
                    "ability": "No Ability",
                    "moves": [
                        "Foul Play",
                        "Swords Dance",
                        "Will-O-Wisp",
                        "Recover"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>, anyhow::Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .with_pass_allowed(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .build(static_local_data_store())
}

#[test]
fn foul_play_uses_target_attack_stat() {
    let mut battle = make_battle(0, krokorok(), krokorok()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Foul Play vs Pass
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Pass vs Swords Dance
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    // Turn 3: Pass vs Recover (Heal to full to prevent fainting)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    // Turn 4: Foul Play vs Pass (should do more damage)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Krokorok,player-1,1|name:Foul Play|target:Krokorok,player-2,1",
            "resisted|mon:Krokorok,player-2,1",
            "split|side:1",
            "damage|mon:Krokorok,player-2,1|health:65/120",
            "damage|mon:Krokorok,player-2,1|health:55/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Krokorok,player-2,1|name:Swords Dance|target:Krokorok,player-2,1",
            "boost|mon:Krokorok,player-2,1|stat:atk|by:2",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Krokorok,player-2,1|name:Recover|target:Krokorok,player-2,1",
            "split|side:1",
            "heal|mon:Krokorok,player-2,1|health:120/120",
            "heal|mon:Krokorok,player-2,1|health:100/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Krokorok,player-1,1|name:Foul Play|target:Krokorok,player-2,1",
            "resisted|mon:Krokorok,player-2,1",
            "split|side:1",
            "damage|mon:Krokorok,player-2,1|health:10/120",
            "damage|mon:Krokorok,player-2,1|health:9/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn foul_play_affected_by_user_status() {
    let mut battle = make_battle(0, krokorok(), krokorok()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Foul Play vs Pass
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Pass vs Will-O-Wisp (burns player 1)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    // Turn 3: Foul Play vs Pass (should do less damage due to burn)
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Krokorok,player-1,1|name:Foul Play|target:Krokorok,player-2,1",
            "resisted|mon:Krokorok,player-2,1",
            "split|side:1",
            "damage|mon:Krokorok,player-2,1|health:65/120",
            "damage|mon:Krokorok,player-2,1|health:55/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Krokorok,player-2,1|name:Will-O-Wisp|target:Krokorok,player-1,1",
            "status|mon:Krokorok,player-1,1|status:Burn",
            "split|side:0",
            "damage|mon:Krokorok,player-1,1|from:status:Burn|health:113/120",
            "damage|mon:Krokorok,player-1,1|from:status:Burn|health:95/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Krokorok,player-1,1|name:Foul Play|target:Krokorok,player-2,1",
            "resisted|mon:Krokorok,player-2,1",
            "split|side:1",
            "damage|mon:Krokorok,player-2,1|health:38/120",
            "damage|mon:Krokorok,player-2,1|health:32/100",
            "split|side:0",
            "damage|mon:Krokorok,player-1,1|from:status:Burn|health:106/120",
            "damage|mon:Krokorok,player-1,1|from:status:Burn|health:89/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
