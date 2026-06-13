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
                    "name": "Toedscruel",
                    "species": "Toedscruel",
                    "ability": "Mycelium Might",
                    "moves": [
                        "Agility",
                        "Tackle",
                        "Flamethrower",
                        "Will-O-Wisp"
                    ],
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
fn mycelium_might_moves_last_in_priority_bracket() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Toedscruel,player-1,1|name:Agility|target:Toedscruel,player-1,1",
            "boost|mon:Toedscruel,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Toedscruel,player-1,1|name:Tackle|target:Toedscruel,player-2,1",
            "split|side:1",
            "damage|mon:Toedscruel,player-2,1|health:121/140",
            "damage|mon:Toedscruel,player-2,1|health:87/100",
            "move|mon:Toedscruel,player-2,1|name:Tackle|target:Toedscruel,player-1,1",
            "split|side:0",
            "damage|mon:Toedscruel,player-1,1|health:122/140",
            "damage|mon:Toedscruel,player-1,1|health:88/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mycelium_might_ignores_abilities_for_status_moves() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Well-Baked Body".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Toedscruel,player-1,1|name:Flamethrower|noanim",
            "boost|mon:Toedscruel,player-2,1|stat:def|by:2|from:ability:Well-Baked Body",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Toedscruel,player-1,1|name:Will-O-Wisp|target:Toedscruel,player-2,1",
            "status|mon:Toedscruel,player-2,1|status:Burn",
            "split|side:1",
            "damage|mon:Toedscruel,player-2,1|from:status:Burn|health:132/140",
            "damage|mon:Toedscruel,player-2,1|from:status:Burn|health:95/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
