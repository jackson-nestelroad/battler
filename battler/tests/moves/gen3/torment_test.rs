use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn nuzleaf() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Nuzleaf",
                    "species": "Nuzleaf",
                    "ability": "No Ability",
                    "moves": [
                        "Torment",
                        "Tackle",
                        "Slash"
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
fn fake_out_only_works_on_first_turn() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, nuzleaf().unwrap(), nuzleaf().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1"),
        Err(err) => assert_eq!(err.full_description(), "cannot move: Nuzleaf's Tackle is disabled")
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2"),
        Err(err) => assert_eq!(err.full_description(), "cannot move: Nuzleaf's Slash is disabled")
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Nuzleaf,player-1,1|name:Torment|target:Nuzleaf,player-2,1",
            "start|mon:Nuzleaf,player-2,1|move:Torment",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Nuzleaf,player-2,1|name:Tackle|target:Nuzleaf,player-1,1",
            "split|side:0",
            "damage|mon:Nuzleaf,player-1,1|health:101/130",
            "damage|mon:Nuzleaf,player-1,1|health:78/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Nuzleaf,player-2,1|name:Slash|target:Nuzleaf,player-1,1",
            "split|side:0",
            "damage|mon:Nuzleaf,player-1,1|health:53/130",
            "damage|mon:Nuzleaf,player-1,1|health:41/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Nuzleaf,player-2,1|name:Tackle|target:Nuzleaf,player-1,1",
            "split|side:0",
            "damage|mon:Nuzleaf,player-1,1|health:23/130",
            "damage|mon:Nuzleaf,player-1,1|health:18/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
