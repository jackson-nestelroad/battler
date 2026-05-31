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
                    "name": "Uxie",
                    "species": "Uxie",
                    "ability": "No Ability",
                    "moves": [
                        "Expanding Force",
                        "Psychic Terrain"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Mesprit",
                    "species": "Mesprit",
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
fn expanding_force_hits_all_foes_in_psychic_terrain() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Uxie,player-1,1|name:Expanding Force|target:Uxie,player-2,1",
            "resisted|mon:Uxie,player-2,1",
            "split|side:1",
            "damage|mon:Uxie,player-2,1|health:231/260",
            "damage|mon:Uxie,player-2,1|health:89/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Uxie,player-1,1|name:Psychic Terrain",
            "fieldstart|move:Psychic Terrain",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Uxie,player-1,1|name:Expanding Force|spread:Uxie,player-2,1;Mesprit,player-2,2",
            "resisted|mon:Uxie,player-2,1",
            "resisted|mon:Mesprit,player-2,2",
            "split|side:1",
            "damage|mon:Uxie,player-2,1|health:192/260",
            "damage|mon:Uxie,player-2,1|health:74/100",
            "split|side:1",
            "damage|mon:Mesprit,player-2,2|health:225/270",
            "damage|mon:Mesprit,player-2,2|health:84/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
