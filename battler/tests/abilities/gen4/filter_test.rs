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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn mimejr() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Mime Jr.",
                    "species": "Mime Jr.",
                    "ability": "Filter",
                    "moves": [
                        "Shadow Claw"
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
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn filter_reduces_damage_from_super_effective_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = mimejr().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(&data, 0, mimejr().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Mime Jr.,player-1,1|name:Shadow Claw|target:Mime Jr.,player-2,1",
            "supereffective|mon:Mime Jr.,player-2,1",
            "split|side:1",
            "damage|mon:Mime Jr.,player-2,1|health:40/80",
            "damage|mon:Mime Jr.,player-2,1|health:50/100",
            "move|mon:Mime Jr.,player-2,1|name:Shadow Claw|target:Mime Jr.,player-1,1",
            "supereffective|mon:Mime Jr.,player-1,1",
            "split|side:0",
            "damage|mon:Mime Jr.,player-1,1|health:50/80",
            "damage|mon:Mime Jr.,player-1,1|health:63/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
