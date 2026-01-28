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

fn hitmonchan() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Hitmonchan",
                    "species": "Hitmonchan",
                    "ability": "No Ability",
                    "moves": [
                        "Wide Guard",
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

fn samurott() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Samurott",
                    "species": "Samurott",
                    "ability": "No Ability",
                    "moves": [
                        "Surf",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn double_team(team: TeamData) -> Result<TeamData> {
    let mut new_team = team.clone();
    new_team.members.push(new_team.members[0].clone());
    Ok(new_team)
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
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
fn wide_guard_blocks_spread_moves() {
    let mut battle = make_battle(
        0,
        double_team(hitmonchan().unwrap()).unwrap(),
        double_team(samurott().unwrap()).unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hitmonchan,player-1,1|name:Wide Guard",
            "singleturn|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "move|mon:Hitmonchan,player-1,2|name:Wide Guard|noanim",
            "fail|mon:Hitmonchan,player-1,2",
            "move|mon:Samurott,player-2,1|name:Surf|spread:Samurott,player-2,2",
            "activate|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "activate|mon:Hitmonchan,player-1,2|move:Wide Guard",
            "resisted|mon:Samurott,player-2,2",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:122/155",
            "damage|mon:Samurott,player-2,2|health:79/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn wide_guard_passes_single_target_moves() {
    let mut battle = make_battle(
        0,
        double_team(hitmonchan().unwrap()).unwrap(),
        double_team(samurott().unwrap()).unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Player 1 uses Wide Guard on both slots.
    // Player 2 uses Tackle on both slots.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hitmonchan,player-1,1|name:Wide Guard",
            "singleturn|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "move|mon:Samurott,player-2,1|name:Tackle|target:Hitmonchan,player-1,1",
            "split|side:0",
            "damage|mon:Hitmonchan,player-1,1|health:87/110",
            "damage|mon:Hitmonchan,player-1,1|health:80/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn wide_guard_succeeds_consecutively() {
    let mut battle = make_battle(
        0,
        double_team(hitmonchan().unwrap()).unwrap(),
        double_team(samurott().unwrap()).unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hitmonchan,player-1,1|name:Wide Guard",
            "singleturn|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "move|mon:Samurott,player-2,1|name:Surf|spread:Samurott,player-2,2",
            "activate|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "activate|mon:Hitmonchan,player-1,2|move:Wide Guard",
            "resisted|mon:Samurott,player-2,2",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:122/155",
            "damage|mon:Samurott,player-2,2|health:79/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Hitmonchan,player-1,1|name:Wide Guard",
            "singleturn|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "move|mon:Samurott,player-2,1|name:Surf|spread:Samurott,player-2,2",
            "activate|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "activate|mon:Hitmonchan,player-1,2|move:Wide Guard",
            "resisted|mon:Samurott,player-2,2",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:92/155",
            "damage|mon:Samurott,player-2,2|health:60/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Hitmonchan,player-1,1|name:Wide Guard",
            "singleturn|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "move|mon:Samurott,player-2,1|name:Surf|spread:Samurott,player-2,2",
            "activate|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "activate|mon:Hitmonchan,player-1,2|move:Wide Guard",
            "resisted|mon:Samurott,player-2,2",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:63/155",
            "damage|mon:Samurott,player-2,2|health:41/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn protect_can_fail_after_wide_guard() {
    let mut battle = make_battle(
        0,
        double_team(hitmonchan().unwrap()).unwrap(),
        double_team(samurott().unwrap()).unwrap(),
    )
    .unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hitmonchan,player-1,1|name:Wide Guard",
            "singleturn|mon:Hitmonchan,player-1,1|move:Wide Guard",
            "move|mon:Samurott,player-2,1|name:Tackle|target:Hitmonchan,player-1,1",
            "split|side:0",
            "damage|mon:Hitmonchan,player-1,1|health:87/110",
            "damage|mon:Hitmonchan,player-1,1|health:80/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Hitmonchan,player-1,1|name:Protect|noanim",
            "fail|mon:Hitmonchan,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
