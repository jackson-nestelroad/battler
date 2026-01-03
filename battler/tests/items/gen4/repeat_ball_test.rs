use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PlayerDex,
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
    static_local_data_store,
};
use hashbrown::HashSet;

fn graveler() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Graveler",
                    "species": "Graveler",
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

fn apply_rng(battle: &mut PublicCoreBattle) {
    let rng = get_controlled_rng_for_battle(battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([
        (1, 65535),
        (2, 0),
        (3, 46811),
        (4, 59293),
        (5, 59294),
    ]);
}

fn make_battle(
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
    dex: PlayerDex,
) -> Result<PublicCoreBattle<'static>> {
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
        .with_player_dex("protagonist", dex)
        .with_team("wild", team_2)
        .build(static_local_data_store())
}

#[test]
fn repeat_ball_does_not_increase_catch_rate_if_species_not_registered() {
    let mut battle = make_battle(
        0,
        graveler().unwrap(),
        graveler().unwrap(),
        PlayerDex::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item repeatball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Repeat Ball|target:Graveler,wild,1",
            "catchfailed|player:protagonist|mon:Graveler,wild,1|item:Repeat Ball|shakes:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn repeat_ball_increases_catch_rate_when_if_species_registered() {
    let mut battle = make_battle(
        0,
        graveler().unwrap(),
        graveler().unwrap(),
        PlayerDex {
            species: HashSet::from_iter(["Graveler".to_owned()]),
        },
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item repeatball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Repeat Ball|target:Graveler,wild,1",
            "catchfailed|player:protagonist|mon:Graveler,wild,1|item:Repeat Ball|shakes:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
