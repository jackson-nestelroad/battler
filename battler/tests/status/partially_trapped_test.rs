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

fn two_gyarados() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gyarados",
                    "species": "Gyarados",
                    "ability": "No Ability",
                    "moves": [
                        "Bind"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Gyarados",
                    "species": "Gyarados",
                    "ability": "No Ability",
                    "moves": [
                        "Bind"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
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
fn bind_partially_traps_target() {
    let mut battle = make_battle(two_gyarados().unwrap(), two_gyarados().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: Gyarados is trapped")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gyarados,player-1,1|name:Bind|target:Gyarados,player-2,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|health:144/155",
            "damage|mon:Gyarados,player-2,1|health:93/100",
            "activate|mon:Gyarados,player-2,1|move:Bind|of:Gyarados,player-1,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:125/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:81/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:106/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:69/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:87/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:57/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:68/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:44/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "end|mon:Gyarados,player-2,1|move:Bind",
            "residual",
            "turn|turn:6",
            ["time"],
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn bind_partially_traps_target_for_longer_with_grip_claw() {
    let mut team = two_gyarados().unwrap();
    team.members[0].item = Some("Grip Claw".to_owned());
    let mut battle = make_battle(team, two_gyarados().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: Gyarados is trapped")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gyarados,player-1,1|name:Bind|target:Gyarados,player-2,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|health:144/155",
            "damage|mon:Gyarados,player-2,1|health:93/100",
            "activate|mon:Gyarados,player-2,1|move:Bind|of:Gyarados,player-1,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:125/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:81/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:106/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:69/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:87/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:57/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:68/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:44/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:49/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:32/100",
            "residual",
            "turn|turn:6",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:30/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:20/100",
            "residual",
            "turn|turn:7",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:11/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:8/100",
            "residual",
            "turn|turn:8",
            ["time"],
            "end|mon:Gyarados,player-2,1|move:Bind",
            "residual",
            "turn|turn:9",
            ["time"],
            "residual",
            "turn|turn:10"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn bind_ends_when_user_switches() {
    let mut battle = make_battle(two_gyarados().unwrap(), two_gyarados().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gyarados,player-1,1|name:Bind|target:Gyarados,player-2,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|health:144/155",
            "damage|mon:Gyarados,player-2,1|health:93/100",
            "activate|mon:Gyarados,player-2,1|move:Bind|of:Gyarados,player-1,1",
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:125/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:81/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:106/155",
            "damage|mon:Gyarados,player-2,1|from:move:Bind|health:69/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Gyarados"],
            ["switch", "player-1", "Gyarados"],
            "end|mon:Gyarados,player-2,1|move:Bind|silent",
            "residual",
            "turn|turn:4",
            ["time"],
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
