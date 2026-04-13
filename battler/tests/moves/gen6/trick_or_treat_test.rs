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
                    "name": "Gourgeist",
                    "species": "Gourgeist",
                    "ability": "No Ability",
                    "moves": [
                        "Trick-or-Treat",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Chesnaught",
                    "species": "Chesnaught",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Reflect Type"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "tera_type": "Ground"
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
fn trick_or_treat_adds_ghost_type() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gourgeist,player-1,1|name:Trick-or-Treat|noanim",
            "fail|mon:Gourgeist,player-1,1",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Chesnaught"],
            ["switch", "player-2", "Chesnaught"],
            "move|mon:Gourgeist,player-1,1|name:Trick-or-Treat|target:Chesnaught,player-2,1",
            "addedtype|mon:Chesnaught,player-2,1|type:Ghost",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Gourgeist,player-1,1|name:Tackle|noanim",
            "immune|mon:Chesnaught,player-2,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn trick_or_treat_added_ghost_type_reflected_by_reflect_type() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chesnaught"],
            ["switch", "player-2", "Chesnaught"],
            "move|mon:Gourgeist,player-1,1|name:Trick-or-Treat|target:Chesnaught,player-2,1",
            "addedtype|mon:Chesnaught,player-2,1|type:Ghost",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Chesnaught"],
            ["switch", "player-1", "Chesnaught"],
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Chesnaught,player-1,1|name:Reflect Type|target:Chesnaught,player-2,1",
            "typechange|mon:Chesnaught,player-1,1|types:Grass/Fighting",
            "addedtype|mon:Chesnaught,player-1,1|type:Ghost",
            "move|mon:Chesnaught,player-2,1|name:Tackle|noanim",
            "immune|mon:Chesnaught,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn terastallization_resists_added_type() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,tera"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Chesnaught"],
            ["switch", "player-2", "Chesnaught"],
            "residual",
            "turn|turn:2",
            "continue",
            "tera|mon:Chesnaught,player-2,1|type:Ground",
            "move|mon:Gourgeist,player-1,1|name:Trick-or-Treat|noanim",
            "fail|mon:Gourgeist,player-1,1",
            "move|mon:Chesnaught,player-2,1|name:Tackle|noanim",
            "immune|mon:Gourgeist,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
