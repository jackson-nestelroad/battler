use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn pelipper() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pelipper",
                    "species": "Pelipper",
                    "ability": "No Ability",
                    "moves": [
                        "Stockpile",
                        "Spit Up",
                        "Swallow"
                    ],
                    "evs": {
                        "spd": 252
                    },
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn stockpile_changes_effect_of_spit_up() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, pelipper().unwrap(), pelipper().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pelipper,player-1,1|name:Stockpile|target:Pelipper,player-1,1",
            "start|mon:Pelipper,player-1,1|move:Stockpile|count:1",
            "boost|mon:Pelipper,player-1,1|stat:def|by:1",
            "boost|mon:Pelipper,player-1,1|stat:spd|by:1",
            "move|mon:Pelipper,player-2,1|name:Stockpile|target:Pelipper,player-2,1",
            "start|mon:Pelipper,player-2,1|move:Stockpile|count:1",
            "boost|mon:Pelipper,player-2,1|stat:def|by:1",
            "boost|mon:Pelipper,player-2,1|stat:spd|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pelipper,player-1,1|name:Stockpile|target:Pelipper,player-1,1",
            "start|mon:Pelipper,player-1,1|move:Stockpile|count:2",
            "boost|mon:Pelipper,player-1,1|stat:def|by:1",
            "boost|mon:Pelipper,player-1,1|stat:spd|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pelipper,player-1,1|name:Stockpile|target:Pelipper,player-1,1",
            "start|mon:Pelipper,player-1,1|move:Stockpile|count:3",
            "boost|mon:Pelipper,player-1,1|stat:def|by:1",
            "boost|mon:Pelipper,player-1,1|stat:spd|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pelipper,player-1,1|name:Stockpile|noanim",
            "fail|mon:Pelipper,player-1,1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pelipper,player-2,1|name:Spit Up|target:Pelipper,player-1,1",
            "split|side:0",
            "damage|mon:Pelipper,player-1,1|health:105/120",
            "damage|mon:Pelipper,player-1,1|health:88/100",
            "unboost|mon:Pelipper,player-2,1|stat:def|by:1",
            "unboost|mon:Pelipper,player-2,1|stat:spd|by:1",
            "end|mon:Pelipper,player-2,1|move:Stockpile",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pelipper,player-1,1|name:Spit Up|target:Pelipper,player-2,1",
            "split|side:1",
            "damage|mon:Pelipper,player-2,1|health:13/120",
            "damage|mon:Pelipper,player-2,1|health:11/100",
            "unboost|mon:Pelipper,player-1,1|stat:def|by:3",
            "unboost|mon:Pelipper,player-1,1|stat:spd|by:3",
            "end|mon:Pelipper,player-1,1|move:Stockpile",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stockpile_changes_effect_of_swallow() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, pelipper().unwrap(), pelipper().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pelipper,player-1,1|name:Stockpile|target:Pelipper,player-1,1",
            "start|mon:Pelipper,player-1,1|move:Stockpile|count:1",
            "boost|mon:Pelipper,player-1,1|stat:def|by:1",
            "boost|mon:Pelipper,player-1,1|stat:spd|by:1",
            "move|mon:Pelipper,player-2,1|name:Stockpile|target:Pelipper,player-2,1",
            "start|mon:Pelipper,player-2,1|move:Stockpile|count:1",
            "boost|mon:Pelipper,player-2,1|stat:def|by:1",
            "boost|mon:Pelipper,player-2,1|stat:spd|by:1",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Pelipper,player-1,1|name:Stockpile|target:Pelipper,player-1,1",
            "start|mon:Pelipper,player-1,1|move:Stockpile|count:2",
            "boost|mon:Pelipper,player-1,1|stat:def|by:1",
            "boost|mon:Pelipper,player-1,1|stat:spd|by:1",
            "residual",
            "turn|turn:8",
            ["time"],
            "move|mon:Pelipper,player-1,1|name:Stockpile|target:Pelipper,player-1,1",
            "start|mon:Pelipper,player-1,1|move:Stockpile|count:3",
            "boost|mon:Pelipper,player-1,1|stat:def|by:1",
            "boost|mon:Pelipper,player-1,1|stat:spd|by:1",
            "residual",
            "turn|turn:9",
            ["time"],
            "move|mon:Pelipper,player-1,1|name:Swallow|target:Pelipper,player-1,1",
            "split|side:0",
            "heal|mon:Pelipper,player-1,1|health:120/120",
            "heal|mon:Pelipper,player-1,1|health:100/100",
            "unboost|mon:Pelipper,player-1,1|stat:def|by:3",
            "unboost|mon:Pelipper,player-1,1|stat:spd|by:3",
            "end|mon:Pelipper,player-1,1|move:Stockpile",
            "move|mon:Pelipper,player-2,1|name:Swallow|target:Pelipper,player-2,1",
            "split|side:1",
            "heal|mon:Pelipper,player-2,1|health:43/120",
            "heal|mon:Pelipper,player-2,1|health:36/100",
            "unboost|mon:Pelipper,player-2,1|stat:def|by:1",
            "unboost|mon:Pelipper,player-2,1|stat:spd|by:1",
            "end|mon:Pelipper,player-2,1|move:Stockpile",
            "residual",
            "turn|turn:10"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 6, &expected_logs);
}
