use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,

    Gender,
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

fn skitty() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Skitty",
                    "species": "Skitty",
                    "ability": "Cute Charm",
                    "moves": [
                        "Tackle"
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
fn cute_charm_has_chance_to_infatuate() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = skitty().unwrap();
    player.members[0].gender = Gender::Male;
    let mut opponent = skitty().unwrap();
    opponent.members[0].gender = Gender::Female;
    let mut battle = make_battle(&data, 0, player, opponent).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Skitty,player-1,1|name:Tackle|target:Skitty,player-2,1",
            "split|side:1",
            "damage|mon:Skitty,player-2,1|health:83/110",
            "damage|mon:Skitty,player-2,1|health:76/100",
            "start|mon:Skitty,player-1,1|move:Attract|from:ability:Cute Charm|of:Skitty,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn cute_charm_fails_for_same_genders() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = skitty().unwrap();
    player.members[0].gender = Gender::Male;
    let mut opponent = skitty().unwrap();
    opponent.members[0].gender = Gender::Male;
    let mut battle = make_battle(&data, 0, player, opponent).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Skitty,player-1,1|name:Tackle|target:Skitty,player-2,1",
            "split|side:1",
            "damage|mon:Skitty,player-2,1|health:83/110",
            "damage|mon:Skitty,player-2,1|health:76/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
