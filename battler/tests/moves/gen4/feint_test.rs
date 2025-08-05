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

fn infernape() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Infernape",
                    "species": "Infernape",
                    "ability": "No Ability",
                    "moves": [
                        "Feint",
                        "Protect"
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
fn feint_breaks_protect() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, infernape().unwrap(), infernape().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Infernape,player-1,1|name:Feint|target:Infernape,player-2,1",
            "split|side:1",
            "damage|mon:Infernape,player-2,1|health:117/136",
            "damage|mon:Infernape,player-2,1|health:87/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Infernape,player-2,1|name:Protect|target:Infernape,player-2,1",
            "singleturn|mon:Infernape,player-2,1|move:Protect",
            "move|mon:Infernape,player-1,1|name:Feint|target:Infernape,player-2,1",
            "activate|mon:Infernape,player-2,1|condition:Break Protect|broken",
            "split|side:1",
            "damage|mon:Infernape,player-2,1|health:99/136",
            "damage|mon:Infernape,player-2,1|health:73/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
