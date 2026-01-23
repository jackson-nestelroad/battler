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

fn team_1() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Starmie",
                    "species": "Starmie",
                    "ability": "Illuminate",
                    "moves": [
                        "Reflect Type",
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Arceus",
                    "species": "Arceus",
                    "ability": "Multitype",
                    "moves": [
                        "Reflect Type"
                    ],
                    "nature": "Hardy",
                    "level": 100
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn team_2() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ferrothorn",
                    "species": "Ferrothorn",
                    "ability": "Iron Barbs",
                    "moves": [
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Tornadus",
                    "species": "Tornadus",
                    "ability": "Prankster",
                    "moves": [
                        "Roost",
                        "Splash"
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
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn reflect_type_success() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Starmie,player-1,1|name:Reflect Type|target:Ferrothorn,player-2,1",
            "typechange|mon:Starmie,player-1,1|types:Grass/Steel",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn reflect_type_fails_on_arceus() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "Arceus"],
            ["switch", "Arceus"],
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Arceus,player-1,1|name:Reflect Type|noanim",
            "fail|mon:Arceus,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn reflect_type_fails_on_terastallized_user() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Terastallize player 1.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0, tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Starmie is now Tera Water. Reflect Type should fail.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Starmie,player-1,1|type:Water",
            "move|mon:Starmie,player-1,1|name:Reflect Type|noanim",
            "fail|mon:Starmie,player-1,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Starmie,player-1,1|name:Reflect Type|noanim",
            "fail|mon:Starmie,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn reflect_type_fails_on_typeless_target() {
    let mut battle = make_battle(0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Player 2 switches to Tornadus.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    // Turn 2: Tornadus uses Roost (becoming typeless), Starmie uses Reflect Type.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "Tornadus"],
            ["switch", "Tornadus"],
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Tornadus,player-2,1|name:Roost|noanim",
            "fail|mon:Tornadus,player-2,1|what:heal",
            "singleturn|mon:Tornadus,player-2,1|move:Roost",
            "move|mon:Starmie,player-1,1|name:Reflect Type|target:Tornadus,player-2,1",
            "typechange|mon:Starmie,player-1,1|types:None",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
