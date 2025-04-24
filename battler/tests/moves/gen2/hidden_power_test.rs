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
    LogMatch,
    TestBattleBuilder,
};

fn unown_hp_dark() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Unown",
                    "species": "Unown",
                    "ability": "No Ability",
                    "moves": [
                        "Hidden Power"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "hidden_power_type": "Dark"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn unown_hp_psychic() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Unown",
                    "species": "Unown",
                    "ability": "No Ability",
                    "moves": [
                        "Hidden Power"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "hidden_power_type": "Psychic"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn hidden_power_uses_hidden_power_type() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        unown_hp_dark().unwrap(),
        unown_hp_psychic().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Unown,player-1,1|name:Hidden Power|target:Unown,player-2,1",
            "supereffective|mon:Unown,player-2,1",
            "split|side:1",
            "damage|mon:Unown,player-2,1|health:32/108",
            "damage|mon:Unown,player-2,1|health:30/100",
            "move|mon:Unown,player-2,1|name:Hidden Power|target:Unown,player-1,1",
            "resisted|mon:Unown,player-1,1",
            "split|side:0",
            "damage|mon:Unown,player-1,1|health:81/108",
            "damage|mon:Unown,player-1,1|health:75/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
