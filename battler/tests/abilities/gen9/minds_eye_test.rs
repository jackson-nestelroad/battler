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
    assert_logs_since_start_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ursaluna",
                    "species": "Ursaluna-Bloodmoon",
                    "ability": "Mind's Eye",
                    "moves": [
                        "Tackle",
                        "Sand Attack"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Gholdengo",
                    "species": "Gholdengo",
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
fn minds_eye_combines_keen_eye_and_foresight() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Ursaluna"],
            ["switch", "player-1", "Ursaluna"],
            "split|side:0",
            ["switch", "player-1", "Gholdengo"],
            ["switch", "player-1", "Gholdengo"],
            "split|side:1",
            ["switch", "player-2", "Ursaluna"],
            ["switch", "player-2", "Ursaluna"],
            "split|side:1",
            ["switch", "player-2", "Gholdengo"],
            ["switch", "player-2", "Gholdengo"],
            "turn|turn:1",
            "continue",
            "move|mon:Ursaluna,player-1,1|name:Tackle|target:Gholdengo,player-2,2",
            "resisted|mon:Gholdengo,player-2,2",
            "split|side:1",
            "damage|mon:Gholdengo,player-2,2|health:137/147",
            "damage|mon:Gholdengo,player-2,2|health:94/100",
            "move|mon:Ursaluna,player-2,1|name:Sand Attack|noanim",
            "fail|mon:Ursaluna,player-1,1|what:unboost|boosts:acc|from:ability:Mind's Eye",
            "fail|mon:Ursaluna,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
