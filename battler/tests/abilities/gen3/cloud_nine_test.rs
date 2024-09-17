use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
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

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cacnea",
                    "species": "Cacnea",
                    "ability": "No Ability",
                    "moves": [
                        "Sandstorm"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Psyduck",
                    "species": "Psyduck",
                    "ability": "Cloud Nine",
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
fn cloud_nine_suppresses_weather() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cacnea,player-1,1|name:Sandstorm",
            "weather|weather:Sandstorm",
            "weather|weather:Sandstorm|residual",
            "split|side:0",
            "damage|mon:Cacnea,player-1,1|from:weather:Sandstorm|health:104/110",
            "damage|mon:Cacnea,player-1,1|from:weather:Sandstorm|health:95/100",
            "split|side:1",
            "damage|mon:Cacnea,player-2,1|from:weather:Sandstorm|health:104/110",
            "damage|mon:Cacnea,player-2,1|from:weather:Sandstorm|health:95/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Psyduck"],
            ["switch", "player-1", "Psyduck"],
            "ability|mon:Psyduck,player-1,1|ability:Cloud Nine",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Cacnea"],
            ["switch", "player-1", "Cacnea"],
            "weather|weather:Sandstorm|residual",
            "split|side:0",
            "damage|mon:Cacnea,player-1,1|from:weather:Sandstorm|health:98/110",
            "damage|mon:Cacnea,player-1,1|from:weather:Sandstorm|health:90/100",
            "split|side:1",
            "damage|mon:Cacnea,player-2,1|from:weather:Sandstorm|health:98/110",
            "damage|mon:Cacnea,player-2,1|from:weather:Sandstorm|health:90/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}