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

fn torchic() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Torchic",
                    "species": "Torchic",
                    "ability": "No Ability",
                    "moves": [
                        "Trick",
                        "Tackle",
                        "Splash"
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
        .with_weather(Some("sandstormweather".to_owned()))
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn trick_switches_items_with_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team_1 = torchic().unwrap();
    team_1.members[0].item = Some("Safety Goggles".to_owned());
    let mut team_2 = torchic().unwrap();
    team_2.members[0].item = Some("Choice Band".to_owned());
    let mut battle = make_battle(&data, 0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Torchic,player-1,1|name:Tackle|target:Torchic,player-2,1",
            "split|side:1",
            "damage|mon:Torchic,player-2,1|health:79/105",
            "damage|mon:Torchic,player-2,1|health:76/100",
            "move|mon:Torchic,player-2,1|name:Tackle|target:Torchic,player-1,1",
            "split|side:0",
            "damage|mon:Torchic,player-1,1|health:70/105",
            "damage|mon:Torchic,player-1,1|health:67/100",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Torchic,player-2,1|from:weather:Sandstorm|health:73/105",
            "damage|mon:Torchic,player-2,1|from:weather:Sandstorm|health:70/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Torchic,player-1,1|name:Trick|target:Torchic,player-2,1",
            "itemend|mon:Torchic,player-2,1|item:Choice Band|from:move:Trick|of:Torchic,player-1,1",
            "itemend|mon:Torchic,player-1,1|item:Safety Goggles|from:move:Trick",
            "item|mon:Torchic,player-1,1|item:Choice Band|from:move:Trick",
            "item|mon:Torchic,player-2,1|item:Safety Goggles|from:move:Trick|of:Torchic,player-1,1",
            "move|mon:Torchic,player-2,1|name:Tackle|target:Torchic,player-1,1",
            "split|side:0",
            "damage|mon:Torchic,player-1,1|health:44/105",
            "damage|mon:Torchic,player-1,1|health:42/100",
            "weather|weather:Sandstorm|residual",
            "split|side:0",
            "damage|mon:Torchic,player-1,1|from:weather:Sandstorm|health:38/105",
            "damage|mon:Torchic,player-1,1|from:weather:Sandstorm|health:37/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Torchic,player-1,1|name:Tackle|target:Torchic,player-2,1",
            "split|side:1",
            "damage|mon:Torchic,player-2,1|health:36/105",
            "damage|mon:Torchic,player-2,1|health:35/100",
            "move|mon:Torchic,player-2,1|name:Splash|target:Torchic,player-2,1",
            "activate|move:Splash",
            "weather|weather:Sandstorm|residual",
            "split|side:0",
            "damage|mon:Torchic,player-1,1|from:weather:Sandstorm|health:32/105",
            "damage|mon:Torchic,player-1,1|from:weather:Sandstorm|health:31/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
