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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn two_venusaur() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "No Ability",
                    "moves": [
                        "Hyper Beam",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "No Ability",
                    "moves": [
                        "Hyper Beam",
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
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(1087134089137400)
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
fn recharge_moves_require_recharge_turn() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, two_venusaur().unwrap(), two_venusaur().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_lock_move_request = serde_json::from_str(
        r#"{
            "team_position": 0,
            "moves": [
                {
                    "name": "Recharge",
                    "id": "recharge",
                    "pp": 0,
                    "max_pp": 0,
                    "target": "User",
                    "disabled": false
                }
            ],
            "trapped": true,
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
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Venusaur does not have a move in slot 1")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Hyper Beam|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:74/140",
            "damage|mon:Venusaur,player-2,1|health:53/100",
            "mustrecharge|mon:Venusaur,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "cant|mon:Venusaur,player-1,1|from:Must Recharge",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:58/140",
            "damage|mon:Venusaur,player-2,1|health:42/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
