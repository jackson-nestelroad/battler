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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
};

fn miltank() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Miltank",
                    "species": "Miltank",
                    "ability": "No Ability",
                    "moves": [
                        "Rollout",
                        "Defense Curl",
                        "Ice Ball"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blissey() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn rollout_doubles_power_for_consecutive_hits() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 12345, miltank().unwrap(), blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:574/620",
            "damage|mon:Blissey,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:483/620",
            "damage|mon:Blissey,player-2,1|health:78/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:302/620",
            "damage|mon:Blissey,player-2,1|health:49/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:0",
            "damage|mon:Blissey,player-2,1|health:0",
            "faint|mon:Blissey,player-2,1",
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Blissey"],
            ["switch", "player-2", "Blissey"],
            "turn|turn:5",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:0",
            "damage|mon:Blissey,player-2,1|health:0",
            "faint|mon:Blissey,player-2,1",
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Blissey"],
            ["switch", "player-2", "Blissey"],
            "turn|turn:6",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:574/620",
            "damage|mon:Blissey,player-2,1|health:93/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rollout_power_resets_if_fails() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, miltank().unwrap(), blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:574/620",
            "damage|mon:Blissey,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|noanim",
            "miss|mon:Blissey,player-2,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:528/620",
            "damage|mon:Blissey,player-2,1|health:86/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rollout_doubles_power_with_defense_curl() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, miltank().unwrap(), blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Miltank,player-1,1|name:Defense Curl|target:Miltank,player-1,1",
            "boost|mon:Miltank,player-1,1|stat:def|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:529/620",
            "damage|mon:Blissey,player-2,1|health:86/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:348/620",
            "damage|mon:Blissey,player-2,1|health:57/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Rollout|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:0",
            "damage|mon:Blissey,player-2,1|health:0",
            "faint|mon:Blissey,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ice_ball_doubles_power_for_consecutive_hits() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 12345, miltank().unwrap(), blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Miltank,player-1,1|name:Ice Ball|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:574/620",
            "damage|mon:Blissey,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Ice Ball|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:483/620",
            "damage|mon:Blissey,player-2,1|health:78/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Ice Ball|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:302/620",
            "damage|mon:Blissey,player-2,1|health:49/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Ice Ball|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:0",
            "damage|mon:Blissey,player-2,1|health:0",
            "faint|mon:Blissey,player-2,1",
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Blissey"],
            ["switch", "player-2", "Blissey"],
            "turn|turn:5",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Ice Ball|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:0",
            "damage|mon:Blissey,player-2,1|health:0",
            "faint|mon:Blissey,player-2,1",
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Blissey"],
            ["switch", "player-2", "Blissey"],
            "turn|turn:6",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Ice Ball|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:574/620",
            "damage|mon:Blissey,player-2,1|health:93/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ice_ball_power_resets_if_fails() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, miltank().unwrap(), blissey().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Miltank,player-1,1|name:Ice Ball|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:574/620",
            "damage|mon:Blissey,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Ice Ball|noanim",
            "miss|mon:Blissey,player-2,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Ice Ball|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:528/620",
            "damage|mon:Blissey,player-2,1|health:86/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
