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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
};

fn dustox() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Dustox",
                    "species": "Dustox",
                    "ability": "No Ability",
                    "moves": [
                        "Fake Out",
                        "Crush Claw",
                        "Thunder Wave"
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Reverse)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn shield_dust_removes_secondary_effects() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = dustox().unwrap();
    team.members[0].ability = "Shield Dust".to_owned();
    let mut battle = make_battle(&data, 0, team, dustox().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dustox,player-2,1|name:Fake Out|target:Dustox,player-1,1",
            "split|side:0",
            "damage|mon:Dustox,player-1,1|health:107/120",
            "damage|mon:Dustox,player-1,1|health:90/100",
            "move|mon:Dustox,player-1,1|name:Fake Out|target:Dustox,player-2,1",
            "split|side:1",
            "damage|mon:Dustox,player-2,1|health:108/120",
            "damage|mon:Dustox,player-2,1|health:90/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Dustox,player-2,1|name:Crush Claw|target:Dustox,player-1,1",
            "split|side:0",
            "damage|mon:Dustox,player-1,1|health:85/120",
            "damage|mon:Dustox,player-1,1|health:71/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Dustox,player-2,1|name:Thunder Wave|target:Dustox,player-1,1",
            "status|mon:Dustox,player-1,1|status:Paralysis",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
