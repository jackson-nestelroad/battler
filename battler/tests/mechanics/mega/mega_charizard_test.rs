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

fn charizard() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_mega_evolution(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn charizard_mega_evolves_based_on_item() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team_1 = charizard().unwrap();
    team_1.members[0].item = Some("Charizardite X".to_owned());
    let mut team_2 = charizard().unwrap();
    team_2.members[0].item = Some("Charizardite Y".to_owned());
    let mut battle = make_battle(&data, 0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,mega"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Charizard-Mega-X"],
            ["specieschange", "player-1", "species:Charizard-Mega-X"],
            "mega|mon:Charizard,player-1,1|species:Charizard-Mega-X|from:item:Charizardite X",
            "split|side:1",
            ["specieschange", "player-2", "species:Charizard-Mega-Y"],
            ["specieschange", "player-2", "species:Charizard-Mega-Y"],
            "mega|mon:Charizard,player-2,1|species:Charizard-Mega-Y|from:item:Charizardite Y",
            "weather|weather:Harsh Sunlight|from:ability:Drought|of:Charizard,player-2,1",
            "move|mon:Charizard,player-1,1|name:Splash|target:Charizard,player-1,1",
            "activate|move:Splash",
            "move|mon:Charizard,player-2,1|name:Splash|target:Charizard,player-2,1",
            "activate|move:Splash",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
