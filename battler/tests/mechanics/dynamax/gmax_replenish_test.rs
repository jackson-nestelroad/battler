use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    static_local_data_store,
};

fn three_snorlax() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Thunder Wave",
                        "Will-O-Wisp"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gigantamax_factor": true
                },
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "gender": "M",
                    "ability": "No Ability",
                    "item": "Rawst Berry",
                    "moves": [
                        "Tackle",
                        "Thunder Wave",
                        "Will-O-Wisp"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gigantamax_factor": true
                },
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "gender": "M",
                    "ability": "No Ability",
                    "item": "Cheri Berry",
                    "moves": [
                        "Tackle",
                        "Thunder Wave",
                        "Will-O-Wisp"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gigantamax_factor": true
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Triples)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_dynamax(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn gmax_replenish_recovers_used_berries_after_use() {
    let mut battle = make_battle(
        0,
        three_snorlax().unwrap(),
        three_snorlax().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;pass;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 1,3;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;pass;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 2,2;pass"),
        Ok(())
    );

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 1)]);

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2,dyna;pass;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snorlax,player-2,2|name:Thunder Wave|target:Snorlax,player-1,3",
            "status|mon:Snorlax,player-1,3|status:Paralysis",
            "itemend|mon:Snorlax,player-1,3|item:Cheri Berry|eat",
            "curestatus|mon:Snorlax,player-1,3|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Snorlax,player-2,2|name:Will-O-Wisp|target:Snorlax,player-1,2",
            "status|mon:Snorlax,player-1,2|status:Burn",
            "itemend|mon:Snorlax,player-1,2|item:Rawst Berry|eat",
            "curestatus|mon:Snorlax,player-1,2|status:Burn|from:item:Rawst Berry",
            "residual",
            "turn|turn:3",
            ["time"],
            "gigantamax|mon:Snorlax,player-1,1|species:Snorlax-Gmax",
            "dynamax|mon:Snorlax,player-1,1",
            "split|side:0",
            "sethp|mon:Snorlax,player-1,1|health:330/330",
            "sethp|mon:Snorlax,player-1,1|health:100/100",
            "move|mon:Snorlax,player-1,1|name:G-Max Replenish|target:Snorlax,player-2,2",
            "split|side:1",
            "damage|mon:Snorlax,player-2,2|health:120/220",
            "damage|mon:Snorlax,player-2,2|health:55/100",
            "item|mon:Snorlax,player-1,2|item:Rawst Berry|from:move:G-Max Replenish|of:Snorlax,player-1,1",
            "item|mon:Snorlax,player-1,3|item:Cheri Berry|from:move:G-Max Replenish|of:Snorlax,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
