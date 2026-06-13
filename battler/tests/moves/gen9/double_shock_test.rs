use anyhow::Result;
use battler::{
    WrapResultError,
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    teams::TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};
use serde_json;

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pawmot",
                    "species": "Pawmot",
                    "ability": "No Ability",
                    "moves": [
                        "Double Shock"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "tera_type": "Electric"
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
fn double_shock_dual_type_loses_electric_type() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pawmot,player-1,1|name:Double Shock|target:Pawmot,player-2,1",
            "resisted|mon:Pawmot,player-2,1",
            "split|side:1",
            "damage|mon:Pawmot,player-2,1|health:130/250",
            "damage|mon:Pawmot,player-2,1|health:52/100",
            "typechange|mon:Pawmot,player-1,1|types:Fighting",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pawmot,player-1,1|name:Double Shock|noanim",
            "fail|mon:Pawmot,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn double_shock_terastallized_remains_electric_type() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Pawmot,player-1,1|type:Electric",
            "move|mon:Pawmot,player-1,1|name:Double Shock|target:Pawmot,player-2,1",
            "resisted|mon:Pawmot,player-2,1",
            "split|side:1",
            "damage|mon:Pawmot,player-2,1|health:90/250",
            "damage|mon:Pawmot,player-2,1|health:36/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pawmot,player-1,1|name:Double Shock|target:Pawmot,player-2,1",
            "resisted|mon:Pawmot,player-2,1",
            "split|side:1",
            "damage|mon:Pawmot,player-2,1|health:0",
            "damage|mon:Pawmot,player-2,1|health:0",
            "faint|mon:Pawmot,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
