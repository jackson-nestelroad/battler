use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    Error,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    assert_turn_logs_eq,
    LogMatch,
    TestBattleBuilder,
};

fn totodile() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Totodile",
                    "species": "Totodile",
                    "ability": "Torrent",
                    "moves": [
                        "Thunderbolt",
                        "Heal Block"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Berry Juice"
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
fn berry_juice_restores_hp_when_consumed() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, totodile().unwrap(), totodile().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Totodile,player-2,1|name:Thunderbolt|target:Totodile,player-1,1",
            "supereffective|mon:Totodile,player-1,1",
            "split|side:0",
            "damage|mon:Totodile,player-1,1|health:38/110",
            "damage|mon:Totodile,player-1,1|health:35/100",
            "itemend|mon:Totodile,player-1,1|item:Berry Juice",
            "split|side:0",
            "heal|mon:Totodile,player-1,1|from:item:Berry Juice|health:58/110",
            "heal|mon:Totodile,player-1,1|from:item:Berry Juice|health:53/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn berry_juice_is_not_used_during_heal_block() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, totodile().unwrap(), totodile().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Totodile,player-2,1|name:Heal Block",
            "start|mon:Totodile,player-1,1|move:Heal Block",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Totodile,player-2,1|name:Thunderbolt|target:Totodile,player-1,1",
            "supereffective|mon:Totodile,player-1,1",
            "split|side:0",
            "damage|mon:Totodile,player-1,1|health:38/110",
            "damage|mon:Totodile,player-1,1|health:35/100",
            "status|mon:Totodile,player-1,1|status:Paralysis",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "end|mon:Totodile,player-1,1|move:Heal Block",
            "residual",
            "itemend|mon:Totodile,player-1,1|item:Berry Juice",
            "split|side:0",
            "heal|mon:Totodile,player-1,1|from:item:Berry Juice|health:58/110",
            "heal|mon:Totodile,player-1,1|from:item:Berry Juice|health:53/100"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&battle, 5, &expected_logs);
}
