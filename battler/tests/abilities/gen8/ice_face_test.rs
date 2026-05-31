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
                    "name": "Eiscue",
                    "species": "Eiscue",
                    "ability": "Ice Face",
                    "moves": [
                        "Fire Punch",
                        "Snowscape"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "Imposter",
                    "moves": [],
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn ice_face_consumes_super_effective_damage() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eiscue,player-1,1|name:Fire Punch|target:Eiscue,player-2,1",
            "split|side:1",
            ["specieschange", "player-2", "Eiscue-Noice"],
            ["specieschange", "player-2", "Eiscue-Noice"],
            "formechange|mon:Eiscue,player-2,1|species:Eiscue-Noice|from:ability:Ice Face",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Eiscue,player-1,1|name:Fire Punch|target:Eiscue,player-2,1",
            "supereffective|mon:Eiscue,player-2,1",
            "split|side:1",
            "damage|mon:Eiscue,player-2,1|health:128/260",
            "damage|mon:Eiscue,player-2,1|health:50/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn transform_takes_on_eiscue_forme_at_transformation() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Ditto"],
            ["switch", "player-2", "Ditto"],
            "transform|mon:Ditto,player-2,1|into:Eiscue,player-1,1|species:Eiscue|from:ability:Imposter",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Eiscue,player-1,1|name:Fire Punch|target:Ditto,player-2,1",
            "supereffective|mon:Ditto,player-2,1",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:114/206",
            "damage|mon:Ditto,player-2,1|health:56/100",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Eiscue"],
            ["switch", "player-2", "Eiscue"],
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Eiscue,player-2,1|name:Fire Punch|target:Eiscue,player-1,1",
            "split|side:0",
            ["specieschange", "player-1", "Eiscue-Noice"],
            ["specieschange", "player-1", "Eiscue-Noice"],
            "formechange|mon:Eiscue,player-1,1|species:Eiscue-Noice|from:ability:Ice Face",
            "residual",
            "turn|turn:5",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Ditto"],
            ["switch", "player-2", "Ditto"],
            "transform|mon:Ditto,player-2,1|into:Eiscue,player-1,1|species:Eiscue-Noice|from:ability:Imposter",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Eiscue,player-1,1|name:Fire Punch|target:Ditto,player-2,1",
            "supereffective|mon:Ditto,player-2,1",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:0",
            "damage|mon:Ditto,player-2,1|health:0",
            "faint|mon:Ditto,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ice_face_resets_when_snow_is_set() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eiscue,player-1,1|name:Fire Punch|target:Eiscue,player-2,1",
            "split|side:1",
            ["specieschange", "player-2", "Eiscue-Noice"],
            ["specieschange", "player-2", "Eiscue-Noice"],
            "formechange|mon:Eiscue,player-2,1|species:Eiscue-Noice|from:ability:Ice Face",
            "move|mon:Eiscue,player-2,1|name:Snowscape",
            "weather|weather:Snow",
            "split|side:1",
            ["specieschange", "player-2", "species:Eiscue|"],
            ["specieschange", "player-2", "species:Eiscue|"],
            "formechange|mon:Eiscue,player-2,1|species:Eiscue|from:ability:Ice Face",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Eiscue,player-1,1|name:Fire Punch|target:Eiscue,player-2,1",
            "split|side:1",
            ["specieschange", "player-2", "Eiscue-Noice"],
            ["specieschange", "player-2", "Eiscue-Noice"],
            "formechange|mon:Eiscue,player-2,1|species:Eiscue-Noice|from:ability:Ice Face",
            "move|mon:Eiscue,player-2,1|name:Snowscape|noanim",
            "fail|mon:Eiscue,player-2,1",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
