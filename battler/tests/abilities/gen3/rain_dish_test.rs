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

fn lotad() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Lotad",
                    "species": "Lotad",
                    "ability": "Rain Dish",
                    "moves": [
                        "Rain Dance",
                        "Tackle"
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
fn rain_dish_heals_in_rain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, lotad().unwrap(), lotad().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lotad,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "move|mon:Lotad,player-2,1|name:Tackle|target:Lotad,player-1,1",
            "split|side:0",
            "damage|mon:Lotad,player-1,1|health:82/100",
            "damage|mon:Lotad,player-1,1|health:82/100",
            "weather|weather:Rain|residual",
            "split|side:0",
            "heal|mon:Lotad,player-1,1|from:ability:Rain Dish|health:88/100",
            "heal|mon:Lotad,player-1,1|from:ability:Rain Dish|health:88/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rain_dish_suppressed_by_utility_umbrella() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = lotad().unwrap();
    player.members[0].item = Some("Utility Umbrella".to_owned());
    let mut battle = make_battle(&data, 0, player, lotad().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lotad,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "move|mon:Lotad,player-2,1|name:Tackle|target:Lotad,player-1,1",
            "split|side:0",
            "damage|mon:Lotad,player-1,1|health:82/100",
            "damage|mon:Lotad,player-1,1|health:82/100",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
