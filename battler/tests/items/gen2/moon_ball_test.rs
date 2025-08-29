use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WildPlayerOptions,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
};

fn cleffa() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cleffa",
                    "species": "Cleffa",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn clefairy() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Clefairy",
                    "species": "Clefairy",
                    "ability": "No Ability",
                    "moves": [],
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
        .with_controlled_rng(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_wild_mon_to_side_2("wild", "Wild", WildPlayerOptions::default())
        .with_team("protagonist", team_1)
        .with_team("wild", team_2)
        .build(data)
}

fn apply_rng(battle: &mut PublicCoreBattle, shake_probability: u64) {
    let rng = get_controlled_rng_for_battle(battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([
        (1, 65535),
        (2, 0),
        (3, 0),
        (4, shake_probability - 1),
        (5, shake_probability),
    ]);
}

#[test]
fn moon_ball_does_not_increase_catch_rate_for_cleffa() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, cleffa().unwrap(), cleffa().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 48891);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item moonball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Moon Ball|target:Cleffa,wild,1",
            "catchfailed|player:protagonist|mon:Cleffa,wild,1|item:Moon Ball|shakes:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn moon_ball_increases_catch_rate_for_clefairy() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, clefairy().unwrap(), clefairy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 63455);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item moonball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Moon Ball|target:Clefairy,wild,1",
            "catchfailed|player:protagonist|mon:Clefairy,wild,1|item:Moon Ball|shakes:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
