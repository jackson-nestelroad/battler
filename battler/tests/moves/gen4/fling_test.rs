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

fn ambipom() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ambipom",
                    "species": "Ambipom",
                    "ability": "No Ability",
                    "moves": [
                        "Fling",
                        "Toxic",
                        "Heal Block",
                        "Growl"
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
fn fling_fails_with_no_item() {
    let mut battle = make_battle(0, ambipom().unwrap(), ambipom().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Fling|noanim",
            "fail|mon:Ambipom,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fling_throws_berry() {
    let mut team = ambipom().unwrap();
    team.members[0].item = Some("Pecha Berry".to_owned());
    let mut battle = make_battle(0, team, ambipom().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Fling|target:Ambipom,player-2,1",
            "activate|mon:Ambipom,player-1,1|move:Fling|item:Pecha Berry",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|health:128/135",
            "damage|mon:Ambipom,player-2,1|health:95/100",
            "itemend|mon:Ambipom,player-1,1|item:Pecha Berry|silent|from:move:Fling",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Ambipom,player-2,1|name:Toxic|target:Ambipom,player-1,1",
            "status|mon:Ambipom,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Ambipom,player-1,1|from:status:Bad Poison|health:127/135",
            "damage|mon:Ambipom,player-1,1|from:status:Bad Poison|health:95/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fling_causes_target_to_eat_berry() {
    let mut team = ambipom().unwrap();
    team.members[0].item = Some("Pecha Berry".to_owned());
    let mut battle = make_battle(0, team, ambipom().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Fling|target:Ambipom,player-2,1",
            "activate|mon:Ambipom,player-1,1|move:Fling|item:Pecha Berry",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|health:120/135",
            "damage|mon:Ambipom,player-2,1|health:89/100",
            "curestatus|mon:Ambipom,player-2,1|status:Bad Poison|from:item:Pecha Berry|of:Ambipom,player-1,1",
            "itemend|mon:Ambipom,player-1,1|item:Pecha Berry|silent|from:move:Fling",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn fling_power_depends_on_item() {
    let mut team = ambipom().unwrap();
    team.members[0].item = Some("Iron Ball".to_owned());
    let mut battle = make_battle(0, team, ambipom().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Fling|target:Ambipom,player-2,1",
            "activate|mon:Ambipom,player-1,1|move:Fling|item:Iron Ball",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|health:52/135",
            "damage|mon:Ambipom,player-2,1|health:39/100",
            "itemend|mon:Ambipom,player-1,1|item:Iron Ball|silent|from:move:Fling",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fling_toxic_orb_triggers_bad_poison() {
    let mut team = ambipom().unwrap();
    team.members[0].item = Some("Toxic Orb".to_owned());
    let mut battle = make_battle(0, team, ambipom().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Fling|target:Ambipom,player-2,1",
            "activate|mon:Ambipom,player-1,1|move:Fling|item:Toxic Orb",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|health:115/135",
            "damage|mon:Ambipom,player-2,1|health:86/100",
            "status|mon:Ambipom,player-2,1|status:Bad Poison",
            "itemend|mon:Ambipom,player-1,1|item:Toxic Orb|silent|from:move:Fling",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|from:status:Bad Poison|health:107/135",
            "damage|mon:Ambipom,player-2,1|from:status:Bad Poison|health:80/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn fling_mental_herb_uses_item() {
    let mut team = ambipom().unwrap();
    team.members[0].item = Some("Mental Herb".to_owned());
    let mut battle = make_battle(0, team, ambipom().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Fling|target:Ambipom,player-2,1",
            "activate|mon:Ambipom,player-1,1|move:Fling|item:Mental Herb",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|health:128/135",
            "damage|mon:Ambipom,player-2,1|health:95/100",
            "end|mon:Ambipom,player-2,1|move:Heal Block",
            "itemend|mon:Ambipom,player-1,1|item:Mental Herb|silent|from:move:Fling",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn fling_white_herb_uses_item() {
    let mut team = ambipom().unwrap();
    team.members[0].item = Some("White Herb".to_owned());
    let mut battle = make_battle(0, team, ambipom().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ambipom,player-1,1|name:Fling|target:Ambipom,player-2,1",
            "activate|mon:Ambipom,player-1,1|move:Fling|item:White Herb",
            "split|side:1",
            "damage|mon:Ambipom,player-2,1|health:128/135",
            "damage|mon:Ambipom,player-2,1|health:95/100",
            "clearnegativeboosts|mon:Ambipom,player-2,1|from:item:White Herb|of:Ambipom,player-1,1",
            "itemend|mon:Ambipom,player-1,1|item:White Herb|silent|from:move:Fling",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
