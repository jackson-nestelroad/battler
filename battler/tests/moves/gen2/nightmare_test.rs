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

fn gengar() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gengar",
                    "species": "Gengar",
                    "ability": "No Ability",
                    "moves": [
                        "Sleep Powder",
                        "Nightmare",
                        "Water Gun"
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
fn nightmare_deals_damage_while_asleep() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, gengar().unwrap(), gengar().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-1,1|name:Sleep Powder|target:Gengar,player-2,1",
            "status|mon:Gengar,player-2,1|status:Sleep",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Gengar,player-1,1|name:Nightmare|target:Gengar,player-2,1",
            "start|mon:Gengar,player-2,1|move:Nightmare",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|from:move:Nightmare|health:90/120",
            "damage|mon:Gengar,player-2,1|from:move:Nightmare|health:75/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "cant|mon:Gengar,player-2,1|from:status:Sleep",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|from:move:Nightmare|health:60/120",
            "damage|mon:Gengar,player-2,1|from:move:Nightmare|health:50/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "cant|mon:Gengar,player-2,1|from:status:Sleep",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|from:move:Nightmare|health:30/120",
            "damage|mon:Gengar,player-2,1|from:move:Nightmare|health:25/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "curestatus|mon:Gengar,player-2,1|status:Sleep",
            "move|mon:Gengar,player-2,1|name:Water Gun|target:Gengar,player-1,1",
            "split|side:0",
            "damage|mon:Gengar,player-1,1|health:93/120",
            "damage|mon:Gengar,player-1,1|health:78/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
