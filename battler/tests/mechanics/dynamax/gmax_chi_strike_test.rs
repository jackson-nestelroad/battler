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
    static_local_data_store,
};

fn machamp() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Machamp",
                    "species": "Machamp",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Focus Energy",
                        "Mach Punch",
                        "Substitute"
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
fn gmax_chi_strike_boosts_crit_ratio_and_stacks() {
    let mut battle = make_battle(0, machamp().unwrap(), machamp().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Machamp,player-1,1|name:Focus Energy|target:Machamp,player-1,1",
            "start|mon:Machamp,player-1,1|move:Focus Energy",
            "move|mon:Machamp,player-2,1|name:Substitute|target:Machamp,player-2,1",
            "start|mon:Machamp,player-2,1|move:Substitute",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:113/150",
            "damage|mon:Machamp,player-2,1|health:76/100",
            "residual",
            "turn|turn:2",
            "continue",
            "gigantamax|mon:Machamp,player-1,1|species:Machamp-Gmax",
            "dynamax|mon:Machamp,player-1,1",
            "split|side:0",
            "sethp|mon:Machamp,player-1,1|health:225/225",
            "sethp|mon:Machamp,player-1,1|health:100/100",
            "move|mon:Machamp,player-1,1|name:G-Max Chi Strike|target:Machamp,player-2,1",
            "crit|mon:Machamp,player-2,1",
            "end|mon:Machamp,player-2,1|move:Substitute",
            "start|mon:Machamp,player-1,1|move:G-Max Chi Strike|count:1",
            "move|mon:Machamp,player-2,1|name:Substitute|target:Machamp,player-2,1",
            "start|mon:Machamp,player-2,1|move:Substitute",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:76/150",
            "damage|mon:Machamp,player-2,1|health:51/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Machamp,player-1,1|name:G-Max Chi Strike|target:Machamp,player-2,1",
            "crit|mon:Machamp,player-2,1",
            "end|mon:Machamp,player-2,1|move:Substitute",
            "start|mon:Machamp,player-1,1|move:G-Max Chi Strike|count:2",
            "move|mon:Machamp,player-2,1|name:Substitute|target:Machamp,player-2,1",
            "start|mon:Machamp,player-2,1|move:Substitute",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:39/150",
            "damage|mon:Machamp,player-2,1|health:26/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Machamp,player-1,1|name:G-Max Chi Strike|target:Machamp,player-2,1",
            "crit|mon:Machamp,player-2,1",
            "end|mon:Machamp,player-2,1|move:Substitute",
            "start|mon:Machamp,player-1,1|move:G-Max Chi Strike|count:3",
            "revertgigantamax|mon:Machamp,player-1,1|species:Machamp",
            "revertdynamax|mon:Machamp,player-1,1",
            "split|side:0",
            "sethp|mon:Machamp,player-1,1|health:150/150",
            "sethp|mon:Machamp,player-1,1|health:100/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
