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
                    "name": "Incineroar",
                    "species": "Incineroar",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Quaquaval",
                    "species": "Quaquaval",
                    "ability": "No Ability",
                    "moves": [
                        "Scald"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Sinistcha",
                    "species": "Sinistcha",
                    "ability": "Hospitality",
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
fn hospitality_heals_ally_on_switch_in() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Quaquaval,player-2,2|name:Scald|target:Incineroar,player-1,1",
            "supereffective|mon:Incineroar,player-1,1",
            "split|side:0",
            "damage|mon:Incineroar,player-1,1|health:57/155",
            "damage|mon:Incineroar,player-1,1|health:37/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Sinistcha"],
            ["switch", "player-1", "Sinistcha"],
            "split|side:0",
            "heal|mon:Incineroar,player-1,1|from:ability:Hospitality|of:Sinistcha,player-1,2|health:95/155",
            "heal|mon:Incineroar,player-1,1|from:ability:Hospitality|of:Sinistcha,player-1,2|health:62/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
