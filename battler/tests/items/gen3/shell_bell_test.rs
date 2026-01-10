use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    static_local_data_store,
};

fn sceptile() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Sceptile",
                    "species": "Sceptile",
                    "ability": "Overgrow",
                    "moves": [
                        "Leaf Blade",
                        "Bullet Seed"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Shell Bell"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn mightyena() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mightyena",
                    "species": "Mightyena",
                    "ability": "Intimidate",
                    "moves": [
                        "Crunch",
                        "Substitute"
                    ],
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn shell_bell_restores_hp_based_on_damage_dealt() {
    let mut battle = make_battle(0, sceptile().unwrap(), mightyena().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mightyena,player-2,1|name:Crunch|target:Sceptile,player-1,1",
            "split|side:0",
            "damage|mon:Sceptile,player-1,1|health:60/130",
            "damage|mon:Sceptile,player-1,1|health:47/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Sceptile,player-1,1|name:Leaf Blade|target:Mightyena,player-2,1",
            "split|side:1",
            "damage|mon:Mightyena,player-2,1|health:85/130",
            "damage|mon:Mightyena,player-2,1|health:66/100",
            "split|side:0",
            "heal|mon:Sceptile,player-1,1|from:item:Shell Bell|health:65/130",
            "heal|mon:Sceptile,player-1,1|from:item:Shell Bell|health:50/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn shell_bell_activates_after_multi_hit_move() {
    let mut battle = make_battle(0, sceptile().unwrap(), mightyena().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 17)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mightyena,player-2,1|name:Crunch|target:Sceptile,player-1,1",
            "split|side:0",
            "damage|mon:Sceptile,player-1,1|health:60/130",
            "damage|mon:Sceptile,player-1,1|health:47/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Sceptile,player-1,1|name:Bullet Seed|target:Mightyena,player-2,1",
            "split|side:1",
            "damage|mon:Mightyena,player-2,1|health:117/130",
            "damage|mon:Mightyena,player-2,1|health:90/100",
            "animatemove|mon:Sceptile,player-1,1|name:Bullet Seed|target:Mightyena,player-2,1",
            "split|side:1",
            "damage|mon:Mightyena,player-2,1|health:104/130",
            "damage|mon:Mightyena,player-2,1|health:80/100",
            "animatemove|mon:Sceptile,player-1,1|name:Bullet Seed|target:Mightyena,player-2,1",
            "split|side:1",
            "damage|mon:Mightyena,player-2,1|health:91/130",
            "damage|mon:Mightyena,player-2,1|health:70/100",
            "animatemove|mon:Sceptile,player-1,1|name:Bullet Seed|target:Mightyena,player-2,1",
            "split|side:1",
            "damage|mon:Mightyena,player-2,1|health:78/130",
            "damage|mon:Mightyena,player-2,1|health:60/100",
            "animatemove|mon:Sceptile,player-1,1|name:Bullet Seed|target:Mightyena,player-2,1",
            "split|side:1",
            "damage|mon:Mightyena,player-2,1|health:65/130",
            "damage|mon:Mightyena,player-2,1|health:50/100",
            "hitcount|hits:5",
            "split|side:0",
            "heal|mon:Sceptile,player-1,1|from:item:Shell Bell|health:68/130",
            "heal|mon:Sceptile,player-1,1|from:item:Shell Bell|health:53/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn shell_bell_activates_after_hitting_substitute() {
    let mut battle = make_battle(0, sceptile().unwrap(), mightyena().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Sceptile,player-1,1|name:Leaf Blade|target:Mightyena,player-2,1",
            "end|mon:Mightyena,player-2,1|move:Substitute",
            "split|side:0",
            "heal|mon:Sceptile,player-1,1|from:item:Shell Bell|health:64/130",
            "heal|mon:Sceptile,player-1,1|from:item:Shell Bell|health:50/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 3, &expected_logs);
}
