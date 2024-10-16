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

fn zangoose() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Zangoose",
                    "species": "Zangoose",
                    "ability": "No Ability",
                    "moves": [
                        "Facade",
                        "Toxic"
                    ],
                    "ivs": {
                        "def": 31
                    },
                    "evs": {
                        "def": 252
                    },
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
fn facade_doubles_power_with_status() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, zangoose().unwrap(), zangoose().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Zangoose,player-1,1|name:Facade|target:Zangoose,player-2,1",
            "split|side:1",
            "damage|mon:Zangoose,player-2,1|health:84/133",
            "damage|mon:Zangoose,player-2,1|health:64/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Zangoose,player-2,1|name:Toxic|target:Zangoose,player-1,1",
            "status|mon:Zangoose,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Zangoose,player-1,1|from:status:Bad Poison|health:125/133",
            "damage|mon:Zangoose,player-1,1|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Zangoose,player-1,1|name:Facade|target:Zangoose,player-2,1",
            "split|side:1",
            "damage|mon:Zangoose,player-2,1|health:0",
            "damage|mon:Zangoose,player-2,1|health:0",
            "faint|mon:Zangoose,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
