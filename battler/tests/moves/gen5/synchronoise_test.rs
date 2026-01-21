use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
        CoreBattleEngineSpeedSortTieResolution,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    static_local_data_store,
    LogMatch,
    TestBattleBuilder,
};

fn heatmor_team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Heatmor",
                    "species": "Heatmor",
                    "ability": "No Ability",
                    "moves": [
                        "Synchronoise",
                        "Burn Up"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(team: TeamData) -> TestBattleBuilder {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team.clone())
        .with_team("player-2", team)
}

#[test]
fn synchronoise_hits_target_with_shared_type() {
    let mut battle = make_battle(heatmor_team()).build(static_local_data_store()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Synchronoise (Hits because of shared Fire type).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Burn Up (User loses Fire type).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Synchronoise (Fails because User is now Typeless).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Heatmor,player-1,1|name:Synchronoise",
            "split|side:1",
            "damage|mon:Heatmor,player-2,1|health:75/145",
            "damage|mon:Heatmor,player-2,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Heatmor,player-1,1|name:Burn Up|target:Heatmor,player-2,1",
            "resisted|mon:Heatmor,player-2,1",
            "split|side:1",
            "damage|mon:Heatmor,player-2,1|health:18/145",
            "damage|mon:Heatmor,player-2,1|health:13/100",
            "typechange|mon:Heatmor,player-1,1|types:None",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Heatmor,player-1,1|name:Synchronoise|noanim",
            "immune|mon:Heatmor,player-2,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}