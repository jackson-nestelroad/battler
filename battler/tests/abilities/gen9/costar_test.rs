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
                    "name": "Flamigo",
                    "species": "Flamigo",
                    "ability": "Costar",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Cetoddle",
                    "species": "Cetoddle",
                    "ability": "No Ability",
                    "moves": [
                        "Swords Dance",
                        "Focus Energy"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Cetitan",
                    "species": "Cetitan",
                    "ability": "No Ability",
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
fn costar_copies_boosts_and_crit_ratio_switch() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2;move 1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 0;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cetoddle,player-1,2|name:Swords Dance|target:Cetoddle,player-1,2",
            "boost|mon:Cetoddle,player-1,2|stat:atk|by:2",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Cetitan"],
            ["switch", "player-1", "Cetitan"],
            "move|mon:Cetoddle,player-1,2|name:Focus Energy|target:Cetoddle,player-1,2",
            "start|mon:Cetoddle,player-1,2|move:Focus Energy",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Flamigo"],
            ["switch", "player-1", "Flamigo"],
            "copyboosts|mon:Flamigo,player-1,1|source:Cetoddle,player-1,2|from:ability:Costar",
            "start|mon:Flamigo,player-1,1|move:Focus Energy|silent",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Flamigo,player-1,1|name:Tackle|target:Flamigo,player-2,1",
            "crit|mon:Flamigo,player-2,1",
            "split|side:1",
            "damage|mon:Flamigo,player-2,1|health:63/142",
            "damage|mon:Flamigo,player-2,1|health:45/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
