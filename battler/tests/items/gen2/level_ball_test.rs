use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
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

fn graveler(level: u8) -> Result<TeamData> {
    let mut team: TeamData = serde_json::from_str(
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
    .wrap_error()?;
    team.members[0].level = level;
    Ok(team)
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
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
fn level_ball_does_not_increase_catch_rate_if_level_less_than_target() {
    let mut battle = make_battle(0, graveler(50).unwrap(), graveler(50).unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 46811);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item levelball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Level Ball|target:Graveler,wild,1",
            "catchfailed|player:protagonist|mon:Graveler,wild,1|item:Level Ball|shakes:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn level_ball_multiples_catch_rate_by_2() {
    let mut battle = make_battle(0, graveler(89).unwrap(), graveler(45).unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 53052);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item levelball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Level Ball|target:Graveler,wild,1",
            "catchfailed|player:protagonist|mon:Graveler,wild,1|item:Level Ball|shakes:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn level_ball_multiples_catch_rate_by_4() {
    let mut battle = make_battle(0, graveler(90).unwrap(), graveler(45).unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 60334);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item levelball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Level Ball|target:Graveler,wild,1",
            "catchfailed|player:protagonist|mon:Graveler,wild,1|item:Level Ball|shakes:3",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn level_ball_multiples_catch_rate_by_8() {
    let mut battle = make_battle(0, graveler(100).unwrap(), graveler(25).unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 65536);

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item levelball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Level Ball|target:Graveler,wild,1",
            "catch|player:protagonist|mon:Graveler,wild,1|item:Level Ball|shakes:4",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
