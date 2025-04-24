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

fn togepi() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Togepi",
                    "species": "Togepi",
                    "ability": "No Ability",
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn hustle_increases_attack_but_decreases_accuracy() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = togepi().unwrap();
    player.members[0].ability = "Hustle".to_owned();
    let mut battle = make_battle(&data, 0, player, togepi().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 0), (5, 90)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Togepi,player-1,1|name:Tackle|target:Togepi,player-2,1",
            "split|side:1",
            "damage|mon:Togepi,player-2,1|health:84/95",
            "damage|mon:Togepi,player-2,1|health:89/100",
            "move|mon:Togepi,player-2,1|name:Tackle|target:Togepi,player-1,1",
            "split|side:0",
            "damage|mon:Togepi,player-1,1|health:87/95",
            "damage|mon:Togepi,player-1,1|health:92/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Togepi,player-1,1|name:Tackle|noanim",
            "miss|mon:Togepi,player-2,1",
            "move|mon:Togepi,player-2,1|name:Tackle|target:Togepi,player-1,1",
            "split|side:0",
            "damage|mon:Togepi,player-1,1|health:79/95",
            "damage|mon:Togepi,player-1,1|health:84/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
