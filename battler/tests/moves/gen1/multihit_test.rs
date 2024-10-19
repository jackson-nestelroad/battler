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
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn make_team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Fury Attack",
                        "Double Kick",
                        "Icicle Spear",
                        "Twineedle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Charmander",
                    "species": "Charmander",
                    "ability": "Blaze",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(data: &dyn DataStore, seed: u64) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", make_team()?)
        .with_team("player-2", make_team()?)
        .build(data)
}

#[test]
fn multihit_number_in_range() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 719270381944144).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:99/105",
            "damage|mon:Bulbasaur,player-2,1|health:95/100",
            "animatemove|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:92/105",
            "damage|mon:Bulbasaur,player-2,1|health:88/100",
            "hitcount|hits:2",
            "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:99/105",
            "damage|mon:Bulbasaur,player-1,1|health:95/100",
            "animatemove|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:92/105",
            "damage|mon:Bulbasaur,player-1,1|health:88/100",
            "animatemove|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:85/105",
            "damage|mon:Bulbasaur,player-1,1|health:81/100",
            "hitcount|hits:3",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:85/105",
            "damage|mon:Bulbasaur,player-2,1|health:81/100",
            "animatemove|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:78/105",
            "damage|mon:Bulbasaur,player-2,1|health:75/100",
            "animatemove|mon:Bulbasaur,player-1,1|name:Fury Attack|target:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:71/105",
            "damage|mon:Bulbasaur,player-2,1|health:68/100",
            "hitcount|hits:3",
            "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:79/105",
            "damage|mon:Bulbasaur,player-1,1|health:76/100",
            "animatemove|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:72/105",
            "damage|mon:Bulbasaur,player-1,1|health:69/100",
            "animatemove|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:65/105",
            "damage|mon:Bulbasaur,player-1,1|health:62/100",
            "hitcount|hits:3",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Bulbasaur,player-1,1|name:Fury Attack|noanim",
            "miss|mon:Bulbasaur,player-2,1",
            "move|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:58/105",
            "damage|mon:Bulbasaur,player-1,1|health:56/100",
            "animatemove|mon:Bulbasaur,player-2,1|name:Fury Attack|target:Bulbasaur,player-1,1",
            "crit|mon:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:48/105",
            "damage|mon:Bulbasaur,player-1,1|health:46/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn multihit_static_number() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 8888888123).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Double Kick|target:Bulbasaur,player-2,1",
            "resisted|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:98/105",
            "damage|mon:Bulbasaur,player-2,1|health:94/100",
            "animatemove|mon:Bulbasaur,player-1,1|name:Double Kick|target:Bulbasaur,player-2,1",
            "resisted|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:92/105",
            "damage|mon:Bulbasaur,player-2,1|health:88/100",
            "hitcount|hits:2",
            "move|mon:Bulbasaur,player-2,1|name:Double Kick|target:Bulbasaur,player-1,1",
            "resisted|mon:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:99/105",
            "damage|mon:Bulbasaur,player-1,1|health:95/100",
            "animatemove|mon:Bulbasaur,player-2,1|name:Double Kick|target:Bulbasaur,player-1,1",
            "resisted|mon:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:93/105",
            "damage|mon:Bulbasaur,player-1,1|health:89/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Bulbasaur,player-1,1|name:Double Kick|target:Bulbasaur,player-2,1",
            "resisted|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:86/105",
            "damage|mon:Bulbasaur,player-2,1|health:82/100",
            "animatemove|mon:Bulbasaur,player-1,1|name:Double Kick|target:Bulbasaur,player-2,1",
            "resisted|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:79/105",
            "damage|mon:Bulbasaur,player-2,1|health:76/100",
            "hitcount|hits:2",
            "move|mon:Bulbasaur,player-2,1|name:Double Kick|target:Bulbasaur,player-1,1",
            "resisted|mon:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:87/105",
            "damage|mon:Bulbasaur,player-1,1|health:83/100",
            "animatemove|mon:Bulbasaur,player-2,1|name:Double Kick|target:Bulbasaur,player-1,1",
            "resisted|mon:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:80/105",
            "damage|mon:Bulbasaur,player-1,1|health:77/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn hit_count_logs_after_faint() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 17)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Icicle Spear|target:Bulbasaur,player-2,1",
            "supereffective|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:35/105",
            "damage|mon:Bulbasaur,player-2,1|health:34/100",
            "animatemove|mon:Bulbasaur,player-1,1|name:Icicle Spear|target:Bulbasaur,player-2,1",
            "supereffective|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:13/105",
            "damage|mon:Bulbasaur,player-2,1|health:13/100",
            "animatemove|mon:Bulbasaur,player-1,1|name:Icicle Spear|target:Bulbasaur,player-2,1",
            "supereffective|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:0",
            "damage|mon:Bulbasaur,player-2,1|health:0",
            "faint|mon:Bulbasaur,player-2,1",
            "hitcount|hits:3",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_turn_logs_eq(&mut battle, 2, &expected_logs);
}

#[test]
fn second_hit_can_apply_secondary_effect() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 756101915254781).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Charmander"],
            ["switch", "player-2", "Charmander"],
            "move|mon:Bulbasaur,player-1,1|name:Twineedle|target:Charmander,player-2,1",
            "resisted|mon:Charmander,player-2,1",
            "split|side:1",
            "damage|mon:Charmander,player-2,1|health:94/99",
            "damage|mon:Charmander,player-2,1|health:95/100",
            "animatemove|mon:Bulbasaur,player-1,1|name:Twineedle|target:Charmander,player-2,1",
            "resisted|mon:Charmander,player-2,1",
            "split|side:1",
            "damage|mon:Charmander,player-2,1|health:88/99",
            "damage|mon:Charmander,player-2,1|health:89/100",
            "status|mon:Charmander,player-2,1|status:Poison",
            "hitcount|hits:2",
            "split|side:1",
            "damage|mon:Charmander,player-2,1|from:status:Poison|health:76/99",
            "damage|mon:Charmander,player-2,1|from:status:Poison|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
