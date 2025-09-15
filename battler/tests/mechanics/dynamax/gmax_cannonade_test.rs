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

fn blastoise() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Surf"
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

fn venusaur() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [],
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
fn gmax_cannonade_damages_non_water_types_at_end_of_turn() {
    let mut battle = make_battle(100, blastoise().unwrap(), venusaur().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Blastoise,player-1,1|species:Blastoise-Gmax",
            "dynamax|mon:Blastoise,player-1,1",
            "split|side:0",
            "sethp|mon:Blastoise,player-1,1|health:208/208",
            "sethp|mon:Blastoise,player-1,1|health:100/100",
            "move|mon:Blastoise,player-1,1|name:G-Max Cannonade|target:Venusaur,player-2,1",
            "resisted|mon:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:102/140",
            "damage|mon:Venusaur,player-2,1|health:73/100",
            "sidestart|side:1|move:G-Max Cannonade",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|from:move:G-Max Cannonade|health:79/140",
            "damage|mon:Venusaur,player-2,1|from:move:G-Max Cannonade|health:57/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
