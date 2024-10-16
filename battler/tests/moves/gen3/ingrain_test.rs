use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    error::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn roselia() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Roselia",
                    "species": "Roselia",
                    "ability": "No Ability",
                    "moves": [
                        "Ingrain",
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Zigzagoon",
                    "species": "Zigzagoon",
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

fn tailow() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Taillow",
                    "species": "Taillow",
                    "ability": "No Ability",
                    "moves": [
                        "Ingrain",
                        "Peck",
                        "Roar"
                    ],
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
) -> Result<PublicCoreBattle, Error> {
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
fn ingrain_heals_user_each_turn() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, roselia().unwrap(), tailow().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Taillow,player-2,1|name:Peck|target:Roselia,player-1,1",
            "supereffective|mon:Roselia,player-1,1",
            "split|side:0",
            "damage|mon:Roselia,player-1,1|health:54/110",
            "damage|mon:Roselia,player-1,1|health:50/100",
            "move|mon:Roselia,player-1,1|name:Ingrain|target:Roselia,player-1,1",
            "start|mon:Roselia,player-1,1|move:Ingrain",
            "split|side:0",
            "heal|mon:Roselia,player-1,1|from:move:Ingrain|health:60/110",
            "heal|mon:Roselia,player-1,1|from:move:Ingrain|health:55/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Roselia,player-1,1|name:Ingrain|noanim",
            "fail|mon:Roselia,player-1,1",
            "split|side:0",
            "heal|mon:Roselia,player-1,1|from:move:Ingrain|health:66/110",
            "heal|mon:Roselia,player-1,1|from:move:Ingrain|health:60/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ingrain_protects_from_forced_switches() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, roselia().unwrap(), tailow().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Roselia,player-1,1|name:Ingrain|target:Roselia,player-1,1",
            "start|mon:Roselia,player-1,1|move:Ingrain",
            "move|mon:Taillow,player-2,1|name:Roar|target:Roselia,player-1,1",
            "activate|mon:Roselia,player-1,1|move:Ingrain",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ingrain_grounds_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, roselia().unwrap(), tailow().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Taillow,player-2,1|name:Ingrain|target:Taillow,player-2,1",
            "start|mon:Taillow,player-2,1|move:Ingrain",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Roselia,player-1,1|name:Earthquake",
            "split|side:1",
            "damage|mon:Taillow,player-2,1|health:20/100",
            "damage|mon:Taillow,player-2,1|health:20/100",
            "split|side:1",
            "heal|mon:Taillow,player-2,1|from:move:Ingrain|health:26/100",
            "heal|mon:Taillow,player-2,1|from:move:Ingrain|health:26/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
