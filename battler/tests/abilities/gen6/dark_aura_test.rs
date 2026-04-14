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
                    "name": "Yveltal",
                    "species": "Yveltal",
                    "ability": "Dark Aura",
                    "moves": [
                        "Bite"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Yveltal",
                    "species": "Yveltal",
                    "ability": "Dark Aura",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Weezing",
                    "species": "Weezing",
                    "ability": "Neutralizing Gas",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Zygarde",
                    "species": "Zygarde",
                    "ability": "Aura Break",
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
fn dark_aura_boosts_dark_type_move_power() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Yveltal,player-2,1|name:Bite|target:Yveltal,player-1,2",
            "resisted|mon:Yveltal,player-1,2",
            "split|side:0",
            "damage|mon:Yveltal,player-1,2|health:294/362",
            "damage|mon:Yveltal,player-1,2|health:82/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Weezing"],
            ["switch", "player-1", "Weezing"],
            "ability|mon:Weezing,player-1,1|ability:Neutralizing Gas",
            "move|mon:Yveltal,player-2,1|name:Bite|target:Yveltal,player-1,2",
            "resisted|mon:Yveltal,player-1,2",
            "split|side:0",
            "damage|mon:Yveltal,player-1,2|health:246/362",
            "damage|mon:Yveltal,player-1,2|health:68/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn aura_break_inverts_dark_aura() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 3;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Zygarde"],
            ["switch", "player-1", "Zygarde"],
            "ability|mon:Zygarde,player-1,1|ability:Aura Break",
            "move|mon:Yveltal,player-2,1|name:Bite|target:Yveltal,player-1,2",
            "resisted|mon:Yveltal,player-1,2",
            "split|side:0",
            "damage|mon:Yveltal,player-1,2|health:324/362",
            "damage|mon:Yveltal,player-1,2|health:90/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
