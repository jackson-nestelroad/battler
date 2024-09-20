use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
        Request,
    },
    common::{
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

fn misdreavus() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Misdreavus",
                    "species": "Misdreavus",
                    "ability": "No Ability",
                    "moves": [
                        "Spite",
                        "Dark Pulse"
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
fn spite_deducts_pp_from_targets_last_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, misdreavus().unwrap(), misdreavus().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-2"), Some(Request::Turn(request)) => {
        assert_eq!(request.active[0].moves[1].pp, 10);
    });

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Misdreavus,player-1,1|name:Spite|noanim",
            "fail|mon:Misdreavus,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Misdreavus,player-2,1|name:Dark Pulse|target:Misdreavus,player-1,1",
            "supereffective|mon:Misdreavus,player-1,1",
            "split|side:0",
            "damage|mon:Misdreavus,player-1,1|health:50/120",
            "damage|mon:Misdreavus,player-1,1|health:42/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Misdreavus,player-1,1|name:Spite|target:Misdreavus,player-2,1",
            "deductpp|mon:Misdreavus,player-2,1|move:Dark Pulse|by:4",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
