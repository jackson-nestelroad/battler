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

fn urshifu() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Urshifu",
                    "species": "Urshifu",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Bite",
                        "Protect"
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
fn gmax_one_blow_breaks_protect() {
    let mut battle = make_battle(100, urshifu().unwrap(), urshifu().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Urshifu,player-1,1|species:Urshifu-Single-Strike-Gmax",
            "dynamax|mon:Urshifu,player-1,1",
            "split|side:0",
            "sethp|mon:Urshifu,player-1,1|health:240/240",
            "sethp|mon:Urshifu,player-1,1|health:100/100",
            "move|mon:Urshifu,player-2,1|name:Protect|target:Urshifu,player-2,1",
            "singleturn|mon:Urshifu,player-2,1|move:Protect",
            "move|mon:Urshifu,player-1,1|name:G-Max One Blow|target:Urshifu,player-2,1",
            "activate|mon:Urshifu,player-2,1|condition:Break Protect|broken",
            "resisted|mon:Urshifu,player-2,1",
            "split|side:1",
            "damage|mon:Urshifu,player-2,1|health:136/160",
            "damage|mon:Urshifu,player-2,1|health:85/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
