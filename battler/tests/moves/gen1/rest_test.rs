use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    Error,
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

fn charizard() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Rest",
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

fn blastoise() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "No Ability",
                    "moves": [
                        "Surf"
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
fn rest_heals_and_causes_sleep_for_three_turns() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, charizard().unwrap(), blastoise().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-1,1|name:Rest|target:Charizard,player-1,1",
            "fail|mon:Charizard,player-1,1|what:heal",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Blastoise,player-2,1|name:Surf",
            "supereffective|mon:Charizard,player-1,1",
            "split|side:0",
            "damage|mon:Charizard,player-1,1|health:22/138",
            "damage|mon:Charizard,player-1,1|health:16/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Rest|target:Charizard,player-1,1",
            "status|mon:Charizard,player-1,1|status:Sleep",
            "split|side:0",
            "heal|mon:Charizard,player-1,1|health:138/138",
            "heal|mon:Charizard,player-1,1|health:100/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "cant|mon:Charizard,player-1,1|reason:status:Sleep",
            "residual",
            "turn|turn:5",
            ["time"],
            "cant|mon:Charizard,player-1,1|reason:status:Sleep",
            "residual",
            "turn|turn:6",
            ["time"],
            "curestatus|mon:Charizard,player-1,1|status:Sleep",
            "move|mon:Charizard,player-1,1|name:Tackle|target:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:125/139",
            "damage|mon:Blastoise,player-2,1|health:90/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
