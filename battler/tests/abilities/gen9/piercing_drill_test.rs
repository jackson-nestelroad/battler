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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Excadrill",
                    "species": "Excadrill",
                    "ability": "No Ability",
                    "item": "Excadrite",
                    "moves": [
                        "Metal Claw",
                        "Protect"
                    ],
                    "nature": "Hardy",
                    "level": 50
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
        .with_mega_evolution(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn piercing_drill_hits_for_quarter_damage_through_protect_on_contact() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Excadrill,player-1,1|name:Metal Claw|target:Excadrill,player-2,1",
            "resisted|mon:Excadrill,player-2,1",
            "split|side:1",
            "damage|mon:Excadrill,player-2,1|health:134/170",
            "damage|mon:Excadrill,player-2,1|health:79/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["specieschange", "player-1", "Excadrill-Mega"],
            ["specieschange", "player-1", "Excadrill-Mega"],
            "mega|mon:Excadrill,player-1,1|species:Excadrill-Mega|from:item:Excadrite",
            "move|mon:Excadrill,player-2,1|name:Protect|target:Excadrill,player-2,1",
            "singleturn|mon:Excadrill,player-2,1|move:Protect",
            "move|mon:Excadrill,player-1,1|name:Metal Claw|target:Excadrill,player-2,1",
            "resisted|mon:Excadrill,player-2,1",
            "activate|mon:Excadrill,player-1,1|ability:Piercing Drill",
            "protectweaken|mon:Excadrill,player-2,1",
            "split|side:1",
            "damage|mon:Excadrill,player-2,1|health:123/170",
            "damage|mon:Excadrill,player-2,1|health:73/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
