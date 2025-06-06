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
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn shroomish() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Shroomish",
                    "species": "Shroomish",
                    "ability": "Effect Spore",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn mudkip() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mudkip",
                    "species": "Mudkip",
                    "ability": "Torrent",
                    "moves": [
                        "Tackle",
                        "Arm Thrust"
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn effect_spore_randomly_inflicts_status_to_attacker_on_contact() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, mudkip().unwrap(), shroomish().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 29)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mudkip,player-1,1|name:Tackle|target:Shroomish,player-2,1",
            "split|side:1",
            "damage|mon:Shroomish,player-2,1|health:99/120",
            "damage|mon:Shroomish,player-2,1|health:83/100",
            "status|mon:Mudkip,player-1,1|status:Poison|from:ability:Effect Spore|of:Shroomish,player-2,1",
            "split|side:0",
            "damage|mon:Mudkip,player-1,1|from:status:Poison|health:97/110",
            "damage|mon:Mudkip,player-1,1|from:status:Poison|health:89/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn effect_spore_causing_attacker_to_fall_asleep_cancels_multi_hit_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, mudkip().unwrap(), shroomish().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(5, 10)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mudkip,player-1,1|name:Arm Thrust|target:Shroomish,player-2,1",
            "split|side:1",
            "damage|mon:Shroomish,player-2,1|health:112/120",
            "damage|mon:Shroomish,player-2,1|health:94/100",
            "status|mon:Mudkip,player-1,1|status:Sleep|from:ability:Effect Spore|of:Shroomish,player-2,1",
            "animatemove|mon:Mudkip,player-1,1|name:Arm Thrust|noanim",
            "hitcount|hits:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
