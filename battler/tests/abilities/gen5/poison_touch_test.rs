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

fn toxicroak() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Toxicroak",
                    "species": "Toxicroak",
                    "ability": "Poison Touch",
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

fn vivillon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Vivillon",
                    "species": "Vivillon",
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
        .with_controlled_rng(true)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn poison_touch_adds_chance_to_poison_on_contact() {
    let mut battle = make_battle(0, toxicroak().unwrap(), vivillon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Toxicroak,player-1,1|name:Tackle|target:Vivillon,player-2,1",
            "split|side:1",
            "damage|mon:Vivillon,player-2,1|health:105/140",
            "damage|mon:Vivillon,player-2,1|health:75/100",
            "status|mon:Vivillon,player-2,1|status:Poison|from:ability:Poison Touch|of:Toxicroak,player-1,1",
            "split|side:1",
            "damage|mon:Vivillon,player-2,1|from:status:Poison|health:88/140",
            "damage|mon:Vivillon,player-2,1|from:status:Poison|health:63/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn shield_dust_prevents_poison_touch() {
    let mut team = vivillon().unwrap();
    team.members[0].ability = "Shield Dust".to_owned();
    let mut battle = make_battle(0, toxicroak().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0), (5, 0), (6, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Toxicroak,player-1,1|name:Tackle|target:Vivillon,player-2,1",
            "split|side:1",
            "damage|mon:Vivillon,player-2,1|health:105/140",
            "damage|mon:Vivillon,player-2,1|health:75/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
