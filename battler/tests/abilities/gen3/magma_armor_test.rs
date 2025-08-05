use anyhow::Result;
use battler::{
    BattleType,
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

fn magcargo() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Magcargo",
                    "species": "Magcargo",
                    "ability": "Magma Armor",
                    "moves": [
                        "Ice Beam",
                        "Skill Swap"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn linoone() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Linoone",
                    "species": "Linoone",
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn magma_armor_prevents_freeze() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, magcargo().unwrap(), linoone().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Magcargo,player-1,1|name:Ice Beam|target:Linoone,player-2,1",
            "split|side:1",
            "damage|mon:Linoone,player-2,1|health:81/138",
            "damage|mon:Linoone,player-2,1|health:59/100",
            "status|mon:Linoone,player-2,1|status:Freeze",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Magcargo,player-1,1|name:Skill Swap|target:Linoone,player-2,1",
            "activate|mon:Linoone,player-2,1|move:Skill Swap|of:Magcargo,player-1,1",
            "abilityend|mon:Magcargo,player-1,1|ability:Magma Armor|from:move:Skill Swap|of:Linoone,player-2,1",
            "ability|mon:Magcargo,player-1,1|ability:No Ability|from:move:Skill Swap|of:Linoone,player-2,1",
            "abilityend|mon:Linoone,player-2,1|ability:No Ability|from:move:Skill Swap|of:Magcargo,player-1,1",
            "ability|mon:Linoone,player-2,1|ability:Magma Armor|from:move:Skill Swap|of:Magcargo,player-1,1",
            "curestatus|mon:Linoone,player-2,1|status:Freeze|from:ability:Magma Armor",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
