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

fn ambipom() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ambipom",
                    "species": "Ambipom",
                    "ability": "Technician",
                    "moves": [
                        "Cut"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn technician_boosts_power_of_move_below_60_base_power() {
    let mut team = ambipom().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, ambipom().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Cut|target:Ambipom,player-2,1",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|health:60/135",
            "damage|mon:Ambipom,player-2,1|health:45/100",
            "move|mon:Ambipom,player-2,1|name:Cut|target:Ambipom,player-1,1",
            "split|side:0",
            "damage|mon:Ambipom,player-1,1|health:84/135",
            "damage|mon:Ambipom,player-1,1|health:63/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn technician_boosts_power_of_move_if_boosted_above_60_base_power() {
    let mut team_1 = ambipom().unwrap();
    team_1.members[0].item = Some("Normal Gem".to_owned());
    let mut team_2 = ambipom().unwrap();
    team_2.members[0].ability = "No Ability".to_owned();
    team_2.members[0].item = Some("Normal Gem".to_owned());
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Cut|target:Ambipom,player-2,1",
            "itemend|mon:Ambipom,player-1,1|item:Normal Gem",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|health:38/135",
            "damage|mon:Ambipom,player-2,1|health:29/100",
            "move|mon:Ambipom,player-2,1|name:Cut|target:Ambipom,player-1,1",
            "itemend|mon:Ambipom,player-2,1|item:Normal Gem",
            "split|side:0",
            "damage|mon:Ambipom,player-1,1|health:69/135",
            "damage|mon:Ambipom,player-1,1|health:52/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
