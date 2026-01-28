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

fn grimmsnarl() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Grimmsnarl",
                    "species": "Grimmsnarl",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Bite"
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
        .with_battle_type(BattleType::Singles)
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
fn gmax_snooze_has_chance_to_inflict_yawn() {
    let mut battle = make_battle(100, grimmsnarl().unwrap(), grimmsnarl().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 0)]);
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(2, 1)]);
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Grimmsnarl,player-1,1|species:Grimmsnarl-Gmax",
            "dynamax|mon:Grimmsnarl,player-1,1",
            "split|side:0",
            "sethp|mon:Grimmsnarl,player-1,1|health:232/232",
            "sethp|mon:Grimmsnarl,player-1,1|health:100/100",
            "move|mon:Grimmsnarl,player-1,1|name:G-Max Snooze|target:Grimmsnarl,player-2,1",
            "resisted|mon:Grimmsnarl,player-2,1",
            "split|side:1",
            "damage|mon:Grimmsnarl,player-2,1|health:122/155",
            "damage|mon:Grimmsnarl,player-2,1|health:79/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Grimmsnarl,player-1,1|name:G-Max Snooze|target:Grimmsnarl,player-2,1",
            "resisted|mon:Grimmsnarl,player-2,1",
            "split|side:1",
            "damage|mon:Grimmsnarl,player-2,1|health:89/155",
            "damage|mon:Grimmsnarl,player-2,1|health:58/100",
            "start|mon:Grimmsnarl,player-2,1|move:Yawn|of:Grimmsnarl,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
