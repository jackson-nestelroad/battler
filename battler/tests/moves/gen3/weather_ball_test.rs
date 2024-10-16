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

fn glalie() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Glalie",
                    "species": "Glalie",
                    "ability": "No Ability",
                    "moves": [
                        "Weather Ball",
                        "Rain Dance",
                        "Snowscape",
                        "Sunny Day"
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
fn weather_ball_changes_type_and_power_by_weather() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, glalie().unwrap(), glalie().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Glalie,player-1,1|name:Weather Ball|target:Glalie,player-2,1",
            "split|side:1",
            "damage|mon:Glalie,player-2,1|health:117/140",
            "damage|mon:Glalie,player-2,1|health:84/100",
            "move|mon:Glalie,player-2,1|name:Rain Dance",
            "weather|weather:Rain",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Glalie,player-1,1|name:Weather Ball|target:Glalie,player-2,1",
            "split|side:1",
            "damage|mon:Glalie,player-2,1|health:55/140",
            "damage|mon:Glalie,player-2,1|health:40/100",
            "move|mon:Glalie,player-2,1|name:Snowscape",
            "weather|weather:Snow",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Glalie,player-1,1|name:Weather Ball|target:Glalie,player-2,1",
            "resisted|mon:Glalie,player-2,1",
            "split|side:1",
            "damage|mon:Glalie,player-2,1|health:26/140",
            "damage|mon:Glalie,player-2,1|health:19/100",
            "move|mon:Glalie,player-2,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Glalie,player-1,1|name:Weather Ball|target:Glalie,player-2,1",
            "supereffective|mon:Glalie,player-2,1",
            "split|side:1",
            "damage|mon:Glalie,player-2,1|health:0",
            "damage|mon:Glalie,player-2,1|health:0",
            "faint|mon:Glalie,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
