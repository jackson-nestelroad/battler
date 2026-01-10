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

fn drifblim() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Drifblim",
                    "species": "Drifblim",
                    "ability": "Unburden",
                    "item": "Cheri Berry",
                    "moves": [
                        "Splash",
                        "Fling",
                        "Gastro Acid"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn porygonz() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Porygon-Z",
                    "species": "Porygon-Z",
                    "ability": "No Ability",
                    "item": "Oran Berry",
                    "moves": [
                        "Splash",
                        "Trick",
                        "Thief",
                        "Thunder Wave",
                        "Embargo"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Porygon-Z",
                    "species": "Porygon-Z",
                    "ability": "Neutralizing Gas",
                    "moves": [
                        "Splash",
                        "Trick",
                        "Thief",
                        "Thunder Wave",
                        "Embargo"
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
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn unburden_activates_if_item_is_taken() {
    let mut team = porygonz().unwrap();
    team.members[0].item = None;
    let mut battle = make_battle(0, drifblim().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Porygon-Z,player-2,1|name:Thief|target:Drifblim,player-1,1",
            "supereffective|mon:Drifblim,player-1,1",
            "split|side:0",
            "damage|mon:Drifblim,player-1,1|health:120/210",
            "damage|mon:Drifblim,player-1,1|health:58/100",
            "itemend|mon:Drifblim,player-1,1|item:Cheri Berry|silent|from:move:Thief|of:Porygon-Z,player-2,1",
            "item|mon:Porygon-Z,player-2,1|item:Cheri Berry|from:move:Thief|of:Drifblim,player-1,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn unburden_activates_if_item_is_flung() {
    let mut battle = make_battle(0, drifblim().unwrap(), porygonz().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Drifblim,player-1,1|name:Fling|target:Porygon-Z,player-2,1",
            "activate|mon:Drifblim,player-1,1|move:Fling|item:Cheri Berry",
            "split|side:1",
            "damage|mon:Porygon-Z,player-2,1|health:140/145",
            "damage|mon:Porygon-Z,player-2,1|health:97/100",
            "itemend|mon:Drifblim,player-1,1|item:Cheri Berry|silent|from:move:Fling",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn unburden_activates_if_item_is_used() {
    let mut battle = make_battle(0, drifblim().unwrap(), porygonz().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Porygon-Z,player-2,1|name:Thunder Wave|target:Drifblim,player-1,1",
            "status|mon:Drifblim,player-1,1|status:Paralysis",
            "itemend|mon:Drifblim,player-1,1|item:Cheri Berry|eat",
            "curestatus|mon:Drifblim,player-1,1|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn unburden_does_not_activate_if_item_is_suppressed() {
    let mut battle = make_battle(0, drifblim().unwrap(), porygonz().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Porygon-Z,player-2,1|name:Embargo|target:Drifblim,player-1,1",
            "start|mon:Drifblim,player-1,1|move:Embargo",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn unburden_does_not_activate_if_ability_is_suppressed_when_item_is_used() {
    let mut battle = make_battle(0, drifblim().unwrap(), porygonz().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "ability|mon:Porygon-Z,player-2,1|ability:Neutralizing Gas",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Porygon-Z,player-2,1|name:Thunder Wave|target:Drifblim,player-1,1",
            "status|mon:Drifblim,player-1,1|status:Paralysis",
            "itemend|mon:Drifblim,player-1,1|item:Cheri Berry|eat",
            "curestatus|mon:Drifblim,player-1,1|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Drifblim,player-1,1|name:Gastro Acid|target:Porygon-Z,player-2,1",
            "abilityend|mon:Porygon-Z,player-2,1|ability:Neutralizing Gas|from:move:Gastro Acid|of:Drifblim,player-1,1",
            "end|mon:Porygon-Z,player-2,1|ability:Neutralizing Gas",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn unburden_reactivates_when_ability_is_unsuppressed() {
    let mut battle = make_battle(0, drifblim().unwrap(), porygonz().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Porygon-Z,player-2,1|name:Thunder Wave|target:Drifblim,player-1,1",
            "status|mon:Drifblim,player-1,1|status:Paralysis",
            "itemend|mon:Drifblim,player-1,1|item:Cheri Berry|eat",
            "curestatus|mon:Drifblim,player-1,1|status:Paralysis|from:item:Cheri Berry",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            "switch|player:player-2|position:1|name:Porygon-Z|health:145/145|species:Porygon-Z|level:50|gender:U",
            "switch|player:player-2|position:1|name:Porygon-Z|health:100/100|species:Porygon-Z|level:50|gender:U",
            "ability|mon:Porygon-Z,player-2,1|ability:Neutralizing Gas",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:5",
            "continue",
            "end|mon:Porygon-Z,player-2,1|ability:Neutralizing Gas",
            "split|side:1",
            "switch|player:player-2|position:1|name:Porygon-Z|health:145/145|species:Porygon-Z|level:50|gender:U",
            "switch|player:player-2|position:1|name:Porygon-Z|health:100/100|species:Porygon-Z|level:50|gender:U",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Drifblim,player-1,1|name:Splash|target:Drifblim,player-1,1",
            "activate|move:Splash",
            "move|mon:Porygon-Z,player-2,1|name:Splash|target:Porygon-Z,player-2,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
