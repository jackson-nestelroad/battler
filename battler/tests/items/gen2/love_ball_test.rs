use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    Gender,
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

fn bulbasaur(gender: Gender) -> Result<TeamData> {
    let mut team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()?;
    team.members[0].gender = gender;
    Ok(team)
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
fn love_ball_increases_catch_rate_for_opposite_genders() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        bulbasaur(Gender::Male).unwrap(),
        bulbasaur(Gender::Female).unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 38489);
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokeball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    apply_rng(&mut battle, 57213);
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item loveball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Poké Ball|target:Bulbasaur,wild,1",
            "catchfailed|player:protagonist|mon:Bulbasaur,wild,1|item:Poké Ball|shakes:3",
            "residual",
            "turn|turn:2",
            ["time"],
            "useitem|player:protagonist|name:Love Ball|target:Bulbasaur,wild,1",
            "catchfailed|player:protagonist|mon:Bulbasaur,wild,1|item:Love Ball|shakes:3",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
