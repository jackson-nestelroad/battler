use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,

    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ralts",
                    "species": "Ralts",
                    "ability": "No Ability",
                    "moves": [
                        "Worry Seed",
                        "Sleep Powder",
                        "Knock Off"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Kirlia",
                    "species": "Kirlia",
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

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
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
        .build(data)
}

#[test]
fn trace_does_not_copy_forbidden_ability() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = team().unwrap();
    player.members[1].ability = "Trace".to_owned();
    let opponent = team().unwrap();
    let mut battle = make_battle(&data, 0, player, opponent).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Kirlia"],
            ["switch", "player-1", "Kirlia"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn trace_copies_target_ability() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = team().unwrap();
    player.members[0].ability = "Trace".to_owned();
    let opponent = team().unwrap();
    let mut battle = make_battle(&data, 0, player, opponent).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ralts,player-1,1|name:Worry Seed|target:Ralts,player-2,1",
            "endability|mon:Ralts,player-2,1|ability:No Ability|from:move:Worry Seed|of:Ralts,player-1,1",
            "ability|mon:Ralts,player-2,1|ability:Insomnia|from:move:Worry Seed|of:Ralts,player-1,1",
            "endability|mon:Ralts,player-1,1|ability:Trace|from:ability:Trace",
            "ability|mon:Ralts,player-1,1|ability:Insomnia|from:ability:Trace",
            "move|mon:Ralts,player-2,1|name:Sleep Powder|noanim",
            "immune|mon:Ralts,player-1,1|from:ability:Insomnia",
            "fail|mon:Ralts,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ability_shield_blocks_trace() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = team().unwrap();
    player.members[0].ability = "Trace".to_owned();
    player.members[0].item = Some("Ability Shield".to_owned());
    let opponent = team().unwrap();
    let mut battle = make_battle(&data, 0, player, opponent).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ralts,player-1,1|name:Worry Seed|target:Ralts,player-2,1",
            "endability|mon:Ralts,player-2,1|ability:No Ability|from:move:Worry Seed|of:Ralts,player-1,1",
            "ability|mon:Ralts,player-2,1|ability:Insomnia|from:move:Worry Seed|of:Ralts,player-1,1",
            "block|mon:Ralts,player-1,1|item:Ability Shield",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ralts,player-2,1|name:Knock Off|target:Ralts,player-1,1",
            "split|side:0",
            "damage|mon:Ralts,player-1,1|health:46/88",
            "damage|mon:Ralts,player-1,1|health:53/100",
            "itemend|mon:Ralts,player-1,1|item:Ability Shield|from:move:Knock Off|of:Ralts,player-2,1",
            "endability|mon:Ralts,player-1,1|ability:Trace|from:ability:Trace",
            "ability|mon:Ralts,player-1,1|ability:Insomnia|from:ability:Trace",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
