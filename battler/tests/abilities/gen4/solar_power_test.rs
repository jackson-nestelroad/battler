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

fn tropius() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Tropius",
                    "species": "Tropius",
                    "ability": "Solar Power",
                    "moves": [
                        "Sunny Day",
                        "Flamethrower"
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
fn solar_power_boosts_special_attack_in_sun() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = tropius().unwrap();
    team.members[0].item = Some("Utility Umbrella".to_owned());
    let mut battle = make_battle(&data, 0, team, tropius().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Tropius,player-1,1|name:Flamethrower|target:Tropius,player-2,1",
            "supereffective|mon:Tropius,player-2,1",
            "split|side:1",
            "damage|mon:Tropius,player-2,1|health:40/159",
            "damage|mon:Tropius,player-2,1|health:26/100",
            "move|mon:Tropius,player-2,1|name:Flamethrower|target:Tropius,player-1,1",
            "supereffective|mon:Tropius,player-1,1",
            "split|side:0",
            "damage|mon:Tropius,player-1,1|health:67/159",
            "damage|mon:Tropius,player-1,1|health:43/100",
            "weather|weather:Harsh Sunlight|residual",
            "split|side:1",
            "damage|mon:Tropius,player-2,1|from:ability:Solar Power|health:21/159",
            "damage|mon:Tropius,player-2,1|from:ability:Solar Power|health:14/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
