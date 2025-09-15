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
    static_local_data_store,
};

fn seel() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Seel",
                    "species": "Seel",
                    "ability": "Ice Body",
                    "moves": [
                        "Hail",
                        "Snowscape",
                        "Thunderbolt"
                    ],
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
fn ice_body_heals_in_hail() {
    let mut battle = make_battle(0, seel().unwrap(), seel().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Seel,player-1,1|name:Thunderbolt|target:Seel,player-2,1",
            "supereffective|mon:Seel,player-2,1",
            "split|side:1",
            "damage|mon:Seel,player-2,1|health:71/125",
            "damage|mon:Seel,player-2,1|health:57/100",
            "weather|weather:Hail|residual",
            "split|side:1",
            "heal|mon:Seel,player-2,1|from:ability:Ice Body|health:78/125",
            "heal|mon:Seel,player-2,1|from:ability:Ice Body|health:63/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn ice_body_heals_in_snow() {
    let mut battle = make_battle(0, seel().unwrap(), seel().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Seel,player-1,1|name:Thunderbolt|target:Seel,player-2,1",
            "supereffective|mon:Seel,player-2,1",
            "split|side:1",
            "damage|mon:Seel,player-2,1|health:71/125",
            "damage|mon:Seel,player-2,1|health:57/100",
            "weather|weather:Snow|residual",
            "split|side:1",
            "heal|mon:Seel,player-2,1|from:ability:Ice Body|health:78/125",
            "heal|mon:Seel,player-2,1|from:ability:Ice Body|health:63/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
