use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn tropius() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Tropius",
                    "species": "Tropius",
                    "ability": "Harvest",
                    "moves": [
                        "Tackle",
                        "Sunny Day"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Chilan Berry"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn harvest_has_chance_to_reclaim_berry_in_sun() {
    let mut battle = make_battle(0, tropius().unwrap(), tropius().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Tropius,player-1,1|name:Tackle|target:Tropius,player-2,1",
            "itemend|mon:Tropius,player-2,1|item:Chilan Berry|eat",
            "activate|mon:Tropius,player-2,1|item:Chilan Berry|weaken",
            "split|side:1",
            "damage|mon:Tropius,player-2,1|health:152/159",
            "damage|mon:Tropius,player-2,1|health:96/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Tropius,player-1,1|name:Tackle|target:Tropius,player-2,1",
            "split|side:1",
            "damage|mon:Tropius,player-2,1|health:138/159",
            "damage|mon:Tropius,player-2,1|health:87/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Tropius,player-2,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "weather|weather:Harsh Sunlight|residual",
            "item|mon:Tropius,player-2,1|item:Chilan Berry|from:ability:Harvest",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
