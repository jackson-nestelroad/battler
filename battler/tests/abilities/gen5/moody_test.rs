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

fn bibarel() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bibarel",
                    "species": "Bibarel",
                    "ability": "Moody",
                    "moves": [],
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
fn moody_boosts_and_drops_random_stats_each_turn() {
    let mut battle = make_battle(0, bibarel().unwrap(), bibarel().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "boost|mon:Bibarel,player-1,1|stat:atk|by:2|from:ability:Moody",
            "unboost|mon:Bibarel,player-1,1|stat:def|by:1|from:ability:Moody",
            "unboost|mon:Bibarel,player-2,1|stat:atk|by:1|from:ability:Moody",
            "boost|mon:Bibarel,player-2,1|stat:def|by:2|from:ability:Moody",
            "residual",
            "turn|turn:2",
            "continue",
            "boost|mon:Bibarel,player-1,1|stat:atk|by:2|from:ability:Moody",
            "unboost|mon:Bibarel,player-1,1|stat:spd|by:1|from:ability:Moody",
            "boost|mon:Bibarel,player-2,1|stat:atk|by:2|from:ability:Moody",
            "unboost|mon:Bibarel,player-2,1|stat:spa|by:1|from:ability:Moody",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
