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
                    "name": "Whismur",
                    "species": "Whismur",
                    "ability": "No Ability",
                    "moves": [
                        "Uproar",
                        "Sleep Powder"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Ralts",
                    "species": "Ralts",
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
fn uproar_continually_prevents_sleep() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Whismur,player-1,1|name:Sleep Powder|target:Whismur,player-2,1",
            "status|mon:Whismur,player-2,1|status:Sleep",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Whismur,player-1,1|name:Uproar|target:Ralts,player-2,2",
            "split|side:1",
            "damage|mon:Ralts,player-2,2|health:12/88",
            "damage|mon:Ralts,player-2,2|health:14/100",
            "start|mon:Whismur,player-1,1|move:Uproar",
            "curestatus|mon:Whismur,player-2,1|status:Sleep|from:move:Uproar|of:Whismur,player-1,1",
            "start|mon:Whismur,player-1,1|move:Uproar|residual",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Whismur,player-1,1|name:Uproar|target:Ralts,player-2,2",
            "split|side:1",
            "damage|mon:Ralts,player-2,2|health:0",
            "damage|mon:Ralts,player-2,2|health:0",
            "faint|mon:Ralts,player-2,2",
            "move|mon:Whismur,player-2,1|name:Sleep Powder|noanim",
            "fail|mon:Whismur,player-1,1|what:status:Sleep|from:move:Uproar",
            "fail|mon:Whismur,player-2,1",
            "start|mon:Whismur,player-1,1|move:Uproar|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
