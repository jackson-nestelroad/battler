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

fn baltoy() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Baltoy",
                    "species": "Baltoy",
                    "ability": "No Ability",
                    "moves": [
                        "Imprison",
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
fn imprison_disables_moves_known_by_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, baltoy().unwrap(), baltoy().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Baltoy,player-1,1|name:Imprison|target:Baltoy,player-1,1",
            "start|mon:Baltoy,player-1,1|move:Imprison",
            "cant|mon:Baltoy,player-2,1|from:move:Imprison",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Baltoy,player-2,1|name:Struggle|target:Baltoy,player-1,1",
            "crit|mon:Baltoy,player-1,1",
            "split|side:0",
            "damage|mon:Baltoy,player-1,1|health:77/100",
            "damage|mon:Baltoy,player-1,1|health:77/100",
            "split|side:1",
            "damage|mon:Baltoy,player-2,1|from:Struggle Recoil|health:75/100",
            "damage|mon:Baltoy,player-2,1|from:Struggle Recoil|health:75/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
