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

fn bulbasaur() -> Result<TeamData> {
    serde_json::from_str(
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
    .wrap_error()
}

fn buzzwole() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Buzzwole",
                    "species": "Buzzwole",
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
fn beast_ball_has_low_catch_rate_for_non_ultra_beast() {
    let mut battle = make_battle(
        0,
        bulbasaur().unwrap(),
        bulbasaur().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 38489);
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokeball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    apply_rng(&mut battle, 24966);
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item beastball"),
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
            "useitem|player:protagonist|name:Beast Ball|target:Bulbasaur,wild,1",
            "catchfailed|player:protagonist|mon:Bulbasaur,wild,1|item:Beast Ball|shakes:3",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn beast_ball_works_for_ultra_beasts() {
    let mut battle = make_battle(
        0,
        bulbasaur().unwrap(),
        buzzwole().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 24966);
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item pokeball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    apply_rng(&mut battle, 24966);
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item ultraball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    apply_rng(&mut battle, 52012);
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item beastball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Poké Ball|target:Buzzwole,wild,1",
            "catchfailed|player:protagonist|mon:Buzzwole,wild,1|item:Poké Ball|shakes:3",
            "residual",
            "turn|turn:2",
            ["time"],
            "useitem|player:protagonist|name:Ultra Ball|target:Buzzwole,wild,1",
            "catchfailed|player:protagonist|mon:Buzzwole,wild,1|item:Ultra Ball|shakes:3",
            "residual",
            "turn|turn:3",
            ["time"],
            "useitem|player:protagonist|name:Beast Ball|target:Buzzwole,wild,1",
            "catchfailed|player:protagonist|mon:Buzzwole,wild,1|item:Beast Ball|shakes:3",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn master_ball_works_for_ultra_beasts() {
    let mut battle = make_battle(
        0,
        bulbasaur().unwrap(),
        buzzwole().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    apply_rng(&mut battle, 65535);
    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item masterball"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Master Ball|target:Buzzwole,wild,1",
            "catch|player:protagonist|mon:Buzzwole,wild,1|item:Master Ball|shakes:4",
            "exp|mon:Bulbasaur,protagonist,1|exp:2851",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
