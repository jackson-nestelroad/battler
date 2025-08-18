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
};

fn cresselia() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Cresselia",
                    "species": "Cresselia",
                    "ability": "No Ability",
                    "moves": [
                        "Psycho Shift",
                        "Toxic"
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
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn psycho_shift_applies_status_to_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, cresselia().unwrap(), cresselia().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cresselia,player-1,1|name:Psycho Shift|noanim",
            "fail|mon:Cresselia,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Cresselia,player-2,1|name:Toxic|target:Cresselia,player-1,1",
            "status|mon:Cresselia,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Cresselia,player-1,1|from:status:Bad Poison|health:169/180",
            "damage|mon:Cresselia,player-1,1|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Cresselia,player-1,1|name:Psycho Shift|target:Cresselia,player-2,1",
            "status|mon:Cresselia,player-2,1|status:Bad Poison",
            "curestatus|mon:Cresselia,player-1,1|status:Bad Poison",
            "split|side:1",
            "damage|mon:Cresselia,player-2,1|from:status:Bad Poison|health:169/180",
            "damage|mon:Cresselia,player-2,1|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
