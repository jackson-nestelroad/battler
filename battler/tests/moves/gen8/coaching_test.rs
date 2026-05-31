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
                    "name": "Urshifu",
                    "species": "Urshifu",
                    "ability": "No Ability",
                    "moves": [
                        "Coaching"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Urshifu",
                    "species": "Urshifu",
                    "ability": "No Ability",
                    "moves": [
                        "Coaching"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Urshifu",
                    "species": "Urshifu",
                    "ability": "No Ability",
                    "moves": [
                        "Crafty Shield"
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
        .with_battle_type(BattleType::Triples)
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
fn coaching_boosts_stats_of_adjacent_allies() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;move 0;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Urshifu,player-1,1|name:Coaching|target:Urshifu,player-1,2",
            "boost|mon:Urshifu,player-1,2|stat:atk|by:1",
            "boost|mon:Urshifu,player-1,2|stat:def|by:1",
            "move|mon:Urshifu,player-1,2|name:Coaching|spread:Urshifu,player-1,1;Urshifu,player-1,3",
            "boost|mon:Urshifu,player-1,1|stat:atk|by:1",
            "boost|mon:Urshifu,player-1,1|stat:def|by:1",
            "boost|mon:Urshifu,player-1,3|stat:atk|by:1",
            "boost|mon:Urshifu,player-1,3|stat:def|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn coaching_bypasses_crafty_shield() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;pass;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Urshifu,player-1,3|name:Crafty Shield",
            "singleturn|mon:Urshifu,player-1,3|move:Crafty Shield",
            "move|mon:Urshifu,player-1,2|name:Coaching|spread:Urshifu,player-1,1;Urshifu,player-1,3",
            "boost|mon:Urshifu,player-1,1|stat:atk|by:1",
            "boost|mon:Urshifu,player-1,1|stat:def|by:1",
            "boost|mon:Urshifu,player-1,3|stat:atk|by:1",
            "boost|mon:Urshifu,player-1,3|stat:def|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
