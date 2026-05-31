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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Kommo-o",
                    "species": "Kommo-o",
                    "ability": "No Ability",
                    "moves": [
                        "Clangorous Soul"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
fn clangorous_soul_raises_stats_and_loses_one_third_hp() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kommo-o,player-1,1|name:Clangorous Soul|target:Kommo-o,player-1,1",
            "boost|mon:Kommo-o,player-1,1|stat:atk|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:def|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spa|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spd|by:1",
            "boost|mon:Kommo-o,player-1,1|stat:spe|by:1",
            "split|side:0",
            "damage|mon:Kommo-o,player-1,1|health:174/260",
            "damage|mon:Kommo-o,player-1,1|health:67/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn clangorous_soul_fails_if_user_hp_is_too_low() {
    let mut team_1 = team().unwrap();
    team_1.members[0].persistent_battle_data.hp = Some(1);
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kommo-o,player-1,1|name:Clangorous Soul|noanim",
            "fail|mon:Kommo-o,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
