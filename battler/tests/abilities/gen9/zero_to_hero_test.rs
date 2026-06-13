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
                    "name": "Palafin",
                    "species": "Palafin",
                    "ability": "Zero to Hero",
                    "moves": [
                        "U-Turn"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Revavroom",
                    "species": "Revavroom",
                    "ability": "No Ability",
                    "moves": [],
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
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn zero_to_hero_transforms_palafin_on_first_switch_out() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Palafin-Hero"],
            ["specieschange", "player-1", "species:Palafin-Hero"],
            "formechange|mon:Palafin,player-1,1|species:Palafin-Hero|from:ability:Zero to Hero",
            "split|side:0",
            ["switch", "player-1", "Revavroom"],
            ["switch", "player-1", "Revavroom"],
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "species:Palafin-Hero"],
            ["switch", "player-1", "species:Palafin-Hero"],
            "activate|mon:Palafin,player-1,1|ability:Zero to Hero",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Revavroom"],
            ["switch", "player-1", "Revavroom"],
            "residual",
            "turn|turn:4",
            "continue",
            "split|side:0",
            ["switch", "player-1", "species:Palafin-Hero"],
            ["switch", "player-1", "species:Palafin-Hero"],
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn zero_to_hero_transform_occurs_after_switch_out() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Palafin,player-1,1|name:U-turn|target:Palafin,player-2,1",
            "split|side:1",
            "damage|mon:Palafin,player-2,1|health:129/160",
            "damage|mon:Palafin,player-2,1|health:81/100",
            "switchout|mon:Palafin,player-1,1",
            "split|side:0",
            ["specieschange", "player-1", "species:Palafin-Hero"],
            ["specieschange", "player-1", "species:Palafin-Hero"],
            "formechange|mon:Palafin,player-1,1|species:Palafin-Hero|from:ability:Zero to Hero",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Revavroom"],
            ["switch", "player-1", "Revavroom"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
