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
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn snivy() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snivy",
                    "species": "Snivy",
                    "ability": "Overgrow",
                    "moves": [
                        "Leaf Blade",
                        "Magical Leaf"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn oshawott() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Oshawott",
                    "species": "Oshawott",
                    "ability": "Torrent",
                    "moves": [
                        "Recycle"
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
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn liechi_berry_boosts_attack() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = oshawott().unwrap();
    team.members[0].item = Some("Liechi Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, snivy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snivy,player-2,1|name:Leaf Blade|target:Oshawott,player-1,1",
            "supereffective|mon:Oshawott,player-1,1",
            "split|side:0",
            "damage|mon:Oshawott,player-1,1|health:13/115",
            "damage|mon:Oshawott,player-1,1|health:12/100",
            "itemend|mon:Oshawott,player-1,1|item:Liechi Berry|eat",
            "boost|mon:Oshawott,player-1,1|stat:atk|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ganlon_berry_boosts_defense() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = oshawott().unwrap();
    team.members[0].item = Some("Ganlon Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, snivy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snivy,player-2,1|name:Leaf Blade|target:Oshawott,player-1,1",
            "supereffective|mon:Oshawott,player-1,1",
            "split|side:0",
            "damage|mon:Oshawott,player-1,1|health:13/115",
            "damage|mon:Oshawott,player-1,1|health:12/100",
            "itemend|mon:Oshawott,player-1,1|item:Ganlon Berry|eat",
            "boost|mon:Oshawott,player-1,1|stat:def|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn starf_berry_boosts_random_stat() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = oshawott().unwrap();
    team.members[0].ability = "Pickup".to_owned();
    team.members[0].item = Some("Starf Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, snivy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 0), (4, 4)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 1), (2, 2)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snivy,player-2,1|name:Leaf Blade|target:Oshawott,player-1,1",
            "supereffective|mon:Oshawott,player-1,1",
            "split|side:0",
            "damage|mon:Oshawott,player-1,1|health:13/115",
            "damage|mon:Oshawott,player-1,1|health:12/100",
            "itemend|mon:Oshawott,player-1,1|item:Starf Berry|eat",
            "boost|mon:Oshawott,player-1,1|stat:atk|by:2",
            "item|mon:Oshawott,player-1,1|item:Starf Berry|from:ability:Pickup",
            "residual",
            "itemend|mon:Oshawott,player-1,1|item:Starf Berry|eat",
            "boost|mon:Oshawott,player-1,1|stat:spe|by:2",
            "turn|turn:2",
            ["time"],
            "move|mon:Oshawott,player-1,1|name:Recycle|target:Oshawott,player-1,1",
            "item|mon:Oshawott,player-1,1|item:Starf Berry|from:move:Recycle",
            "itemend|mon:Oshawott,player-1,1|item:Starf Berry|eat",
            "boost|mon:Oshawott,player-1,1|stat:def|by:2",
            "item|mon:Oshawott,player-1,1|item:Starf Berry|from:ability:Pickup",
            "residual",
            "itemend|mon:Oshawott,player-1,1|item:Starf Berry|eat",
            "boost|mon:Oshawott,player-1,1|stat:spa|by:2",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn kee_berry_boosts_defense_after_hit_by_physical_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = oshawott().unwrap();
    team.members[0].item = Some("Kee Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, snivy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snivy,player-2,1|name:Leaf Blade|target:Oshawott,player-1,1",
            "supereffective|mon:Oshawott,player-1,1",
            "split|side:0",
            "damage|mon:Oshawott,player-1,1|health:13/115",
            "damage|mon:Oshawott,player-1,1|health:12/100",
            "itemend|mon:Oshawott,player-1,1|item:Kee Berry|eat",
            "boost|mon:Oshawott,player-1,1|stat:def|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn maranga_berry_boosts_special_defense_after_hit_by_special_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = oshawott().unwrap();
    team.members[0].item = Some("Maranga Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, snivy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snivy,player-2,1|name:Magical Leaf|target:Oshawott,player-1,1",
            "supereffective|mon:Oshawott,player-1,1",
            "crit|mon:Oshawott,player-1,1",
            "split|side:0",
            "damage|mon:Oshawott,player-1,1|health:11/115",
            "damage|mon:Oshawott,player-1,1|health:10/100",
            "itemend|mon:Oshawott,player-1,1|item:Maranga Berry|eat",
            "boost|mon:Oshawott,player-1,1|stat:spd|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
