use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn delibird() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Delibird",
                    "species": "Delibird",
                    "ability": "No Ability",
                    "moves": [
                        "Present"
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn present_deals_damage_or_heals_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, delibird().unwrap(), delibird().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([
        (1, 9),
        (2, 9),
        (3, 9),
        (4, 6),
        (8, 9),
        (9, 5),
        (10, 0),
    ]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Delibird,player-1,1|name:Present|target:Delibird,player-2,1",
            "split|side:1",
            "damage|mon:Delibird,player-2,1|health:40/105",
            "damage|mon:Delibird,player-2,1|health:39/100",
            "move|mon:Delibird,player-2,1|name:Present|target:Delibird,player-1,1",
            "split|side:0",
            "damage|mon:Delibird,player-1,1|health:61/105",
            "damage|mon:Delibird,player-1,1|health:59/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Delibird,player-1,1|name:Present|target:Delibird,player-2,1",
            "split|side:1",
            "damage|mon:Delibird,player-2,1|health:17/105",
            "damage|mon:Delibird,player-2,1|health:17/100",
            "move|mon:Delibird,player-2,1|name:Present|target:Delibird,player-1,1",
            "split|side:0",
            "heal|mon:Delibird,player-1,1|health:87/105",
            "heal|mon:Delibird,player-1,1|health:83/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
