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
                    "name": "Slowking",
                    "species": "Slowking-Galar",
                    "ability": "Curious Medicine",
                    "moves": [
                        "Growl"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Slowking",
                    "species": "Slowking-Galar",
                    "ability": "No Ability",
                    "moves": [
                        "Agility"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Slowking",
                    "species": "Slowking-Galar",
                    "ability": "No Ability",
                    "moves": [],
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
fn curious_medicine_clears_stat_boosts() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 0;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Slowking"],
            ["switch", "player-2", "Slowking"],
            "move|mon:Slowking,player-1,1|name:Growl|spread:Slowking,player-2,1;Slowking,player-2,2",
            "unboost|mon:Slowking,player-2,1|stat:atk|by:1",
            "unboost|mon:Slowking,player-2,2|stat:atk|by:1",
            "move|mon:Slowking,player-2,2|name:Agility|target:Slowking,player-2,2",
            "boost|mon:Slowking,player-2,2|stat:spe|by:2",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Slowking"],
            ["switch", "player-2", "Slowking"],
            "clearboosts|mon:Slowking,player-2,2|from:ability:Curious Medicine|of:Slowking,player-2,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
