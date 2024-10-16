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

fn gengar() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gengar",
                    "species": "Gengar",
                    "ability": "No Ability",
                    "moves": [
                        "Curse"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn forretress() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Forretress",
                    "species": "Forretress",
                    "ability": "No Ability",
                    "moves": [
                        "Curse"
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
fn ghost_type_curse_applies_curse_to_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, gengar().unwrap(), forretress().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-1,1|name:Curse|target:Forretress,player-2,1",
            "start|mon:Forretress,player-2,1|move:Curse",
            "split|side:0",
            "damage|mon:Gengar,player-1,1|health:60/120",
            "damage|mon:Gengar,player-1,1|health:50/100",
            "split|side:1",
            "damage|mon:Forretress,player-2,1|from:move:Curse|health:102/135",
            "damage|mon:Forretress,player-2,1|from:move:Curse|health:76/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            "damage|mon:Forretress,player-2,1|from:move:Curse|health:69/135",
            "damage|mon:Forretress,player-2,1|from:move:Curse|health:52/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:1",
            "damage|mon:Forretress,player-2,1|from:move:Curse|health:36/135",
            "damage|mon:Forretress,player-2,1|from:move:Curse|health:27/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:1",
            "damage|mon:Forretress,player-2,1|from:move:Curse|health:3/135",
            "damage|mon:Forretress,player-2,1|from:move:Curse|health:3/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn non_ghost_type_curse_affects_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, gengar().unwrap(), forretress().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Forretress,player-2,1|name:Curse|target:Gengar,player-1,1",
            "boost|mon:Forretress,player-2,1|stat:atk|by:1",
            "boost|mon:Forretress,player-2,1|stat:def|by:1",
            "unboost|mon:Forretress,player-2,1|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
