use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn blissey() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
                    "moves": [
                        "Thrash",
                        "Tackle"
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
fn thrash_locks_move_and_confuses_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        20598204958240985,
        blissey().unwrap(),
        blissey().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_lock_move_request = serde_json::from_str(
        r#"{
            "team_position": 0,
            "moves": [
                {
                    "name": "Thrash",
                    "id": "thrash",
                    "pp": 0,
                    "max_pp": 0,
                    "disabled": false
                }
            ],
            "locked_into_move": true
        }"#,
    )
    .unwrap();
    assert_eq!(
        battle
            .request_for_player("player-1")
            .unwrap()
            .map(|req| if let Request::Turn(req) = req {
                req.active.get(0).cloned()
            } else {
                None
            })
            .flatten(),
        Some(expected_lock_move_request)
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Blissey does not have a move in slot 1")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blissey,player-1,1|name:Thrash|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:236/315",
            "damage|mon:Blissey,player-2,1|health:75/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blissey,player-1,1|name:Thrash|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:160/315",
            "damage|mon:Blissey,player-2,1|health:51/100",
            "start|mon:Blissey,player-1,1|condition:Confusion|fatigue",
            "move|mon:Blissey,player-2,1|name:Tackle|target:Blissey,player-1,1",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|health:290/315",
            "damage|mon:Blissey,player-1,1|health:93/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "activate|mon:Blissey,player-1,1|condition:Confusion",
            "split|side:0",
            "damage|mon:Blissey,player-1,1|from:Confusion|health:274/315",
            "damage|mon:Blissey,player-1,1|from:Confusion|health:87/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
