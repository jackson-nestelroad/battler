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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn manaphy() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Manaphy",
                    "species": "Manaphy",
                    "ability": "Hydration",
                    "moves": [
                        "Rain Dance",
                        "Thunder Wave"
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
) -> Result<PublicCoreBattle<'_>> {
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
fn hydration_cures_status_in_rain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, manaphy().unwrap(), manaphy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Manaphy,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "move|mon:Manaphy,player-2,1|name:Thunder Wave|target:Manaphy,player-1,1",
            "status|mon:Manaphy,player-1,1|status:Paralysis",
            "weather|weather:Rain|residual",
            "activate|mon:Manaphy,player-1,1|ability:Hydration",
            "curestatus|mon:Manaphy,player-1,1|status:Paralysis|from:ability:Hydration",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn hydration_does_not_activate_if_weather_is_suppressed() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = manaphy().unwrap();
    team.members[0].item = Some("Utility Umbrella".to_owned());
    let mut battle = make_battle(&data, 0, team, manaphy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Manaphy,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "move|mon:Manaphy,player-2,1|name:Thunder Wave|target:Manaphy,player-1,1",
            "status|mon:Manaphy,player-1,1|status:Paralysis",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
