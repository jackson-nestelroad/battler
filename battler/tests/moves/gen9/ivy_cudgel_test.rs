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
                    "name": "Ogerpon",
                    "species": "Ogerpon",
                    "ability": "No Ability",
                    "moves": [
                        "Ivy Cudgel"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn ivy_cudgel_changes_type_based_on_ogerpon_forme() {
    let mut team_1 = team().unwrap();
    team_1.members[0].item = Some("Hearthflame Mask".to_owned());
    let mut team_2 = team().unwrap();
    team_2.members[0].item = Some("Wellspring Mask".to_owned());
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,tera"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Ogerpon,player-2,1|type:Water",
            "split|side:1",
            ["specieschange", "player-2", "Ogerpon-Wellspring-Mask-Tera"],
            ["specieschange", "player-2", "Ogerpon-Wellspring-Mask-Tera"],
            "formechange|mon:Ogerpon,player-2,1|species:Ogerpon-Wellspring-Mask-Tera|from:species:Ogerpon-Wellspring-Mask",
            "boost|mon:Ogerpon,player-2,1|stat:spd|by:1|from:ability:Embody Aspect",
            "move|mon:Ogerpon,player-1,1|name:Ivy Cudgel|target:Ogerpon,player-2,1|anim:Fire",
            "resisted|mon:Ogerpon,player-2,1",
            "split|side:1",
            "damage|mon:Ogerpon,player-2,1|health:166/270",
            "damage|mon:Ogerpon,player-2,1|health:62/100",
            "move|mon:Ogerpon,player-2,1|name:Ivy Cudgel|target:Ogerpon,player-1,1|anim:Water",
            "crit|mon:Ogerpon,player-1,1",
            "split|side:0",
            "damage|mon:Ogerpon,player-1,1|health:0",
            "damage|mon:Ogerpon,player-1,1|health:0",
            "faint|mon:Ogerpon,player-1,1",
            "win|side:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
