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
                    "name": "Dondozo",
                    "species": "Dondozo",
                    "ability": "No Ability",
                    "moves": [
                        "Order Up"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Quaxly",
                    "species": "Quaxly",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Tatsugiri",
                    "species": "Tatsugiri-Stretchy",
                    "ability": "Commander",
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
        .with_battle_type(BattleType::Doubles)
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
fn order_up_boosts_stats_when_commanded() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Dondozo,player-1,1|name:Order Up|target:Dondozo,player-2,1",
            "split|side:1",
            "damage|mon:Dondozo,player-2,1|health:179/210",
            "damage|mon:Dondozo,player-2,1|health:86/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Tatsugiri"],
            ["switch", "player-1", "Tatsugiri"],
            "activate|mon:Tatsugiri,player-1,2|ability:Commander",
            "start|mon:Tatsugiri,player-1,2|condition:Commanding|of:Dondozo,player-1,1",
            "boost|mon:Dondozo,player-1,1|stat:atk|by:2|from:ability:Commander|of:Tatsugiri,player-1,2",
            "boost|mon:Dondozo,player-1,1|stat:def|by:2|from:ability:Commander|of:Tatsugiri,player-1,2",
            "boost|mon:Dondozo,player-1,1|stat:spa|by:2|from:ability:Commander|of:Tatsugiri,player-1,2",
            "boost|mon:Dondozo,player-1,1|stat:spd|by:2|from:ability:Commander|of:Tatsugiri,player-1,2",
            "boost|mon:Dondozo,player-1,1|stat:spe|by:2|from:ability:Commander|of:Tatsugiri,player-1,2",
            "move|mon:Dondozo,player-1,1|name:Order Up|target:Dondozo,player-2,1",
            "split|side:1",
            "damage|mon:Dondozo,player-2,1|health:123/210",
            "damage|mon:Dondozo,player-2,1|health:59/100",
            "boost|mon:Dondozo,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
