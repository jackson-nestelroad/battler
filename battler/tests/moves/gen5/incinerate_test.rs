use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    static_local_data_store,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pansear",
                    "species": "Pansear",
                    "ability": "No Ability",
                    "moves": ["Incinerate"],
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn target_team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ducklett",
                    "species": "Ducklett",
                    "ability": "No Ability",
                    "moves": [],
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(seed: u64) -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_seed(seed)
        .with_battle_type(BattleType::Singles)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
}

#[test]
fn incinerate_destroys_berry() {
    let mut team_2 = target_team();
    team_2.members[0].item = Some("Oran Berry".to_string());

    let mut battle = make_battle(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team())
        .with_team("player-2", team_2)
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Incinerate",
            "resisted|mon:Ducklett,player-2,1",
            "split|side:1",
            "damage|mon:Ducklett,player-2,1|health:101/122",
            "damage|mon:Ducklett,player-2,1|health:83/100",
            "itemend|mon:Ducklett,player-2,1|item:Oran Berry|from:move:Incinerate|of:Pansear,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn incinerate_destroys_gem() {
    let mut team_2 = target_team();
    team_2.members[0].item = Some("Fire Gem".to_string());

    let mut battle = make_battle(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team())
        .with_team("player-2", team_2)
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Incinerate",
            "resisted|mon:Ducklett,player-2,1",
            "split|side:1",
            "damage|mon:Ducklett,player-2,1|health:101/122",
            "damage|mon:Ducklett,player-2,1|health:83/100",
            "itemend|mon:Ducklett,player-2,1|item:Fire Gem|from:move:Incinerate|of:Pansear,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn incinerate_does_not_destroy_other_items() {
    let mut team_2 = target_team();
    team_2.members[0].item = Some("Leftovers".to_string());

    let mut battle = make_battle(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team())
        .with_team("player-2", team_2)
        .build(static_local_data_store())
        .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Incinerate",
            "resisted|mon:Ducklett,player-2,1",
            "split|side:1",
            "damage|mon:Ducklett,player-2,1|health:101/122",
            "damage|mon:Ducklett,player-2,1|health:83/100",
            "split|side:1",
            "heal|mon:Ducklett,player-2,1|from:item:Leftovers|health:108/122",
            "heal|mon:Ducklett,player-2,1|from:item:Leftovers|health:89/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}