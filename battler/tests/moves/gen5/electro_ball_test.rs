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

fn galvantula() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Galvantula",
                    "species": "Galvantula",
                    "ability": "No Ability",
                    "moves": [
                        "Electro Ball",
                        "Agility",
                        "Recover"
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
fn electro_ball_damage_scaling() {
    let mut battle = make_battle(0, galvantula().unwrap(), galvantula().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Equal Speed (Ratio 1). Expect 60 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: P1 Agility (+2 Speed). P2 Recover.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    // Turn 3: Ratio 2 (P1 2x Speed). Expect 80 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 4: P1 Agility (+4 Speed). P2 Recover.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    // Turn 5: Ratio 3 (P1 3x Speed). Expect 120 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 6: P1 Agility (+6 Speed). P2 Recover.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    // Turn 7: Ratio 4 (P1 4x Speed). Expect 150 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Galvantula,player-1,1|name:Electro Ball|target:Galvantula,player-2,1",
            "resisted|mon:Galvantula,player-2,1",
            "split|side:1",
            "damage|mon:Galvantula,player-2,1|health:100/130",
            "damage|mon:Galvantula,player-2,1|health:77/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Galvantula,player-1,1|name:Agility|target:Galvantula,player-1,1",
            "boost|mon:Galvantula,player-1,1|stat:spe|by:2",
            "move|mon:Galvantula,player-2,1|name:Recover|target:Galvantula,player-2,1",
            "split|side:1",
            "heal|mon:Galvantula,player-2,1|health:130/130",
            "heal|mon:Galvantula,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Galvantula,player-1,1|name:Electro Ball|target:Galvantula,player-2,1",
            "resisted|mon:Galvantula,player-2,1",
            "split|side:1",
            "damage|mon:Galvantula,player-2,1|health:92/130",
            "damage|mon:Galvantula,player-2,1|health:71/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Galvantula,player-1,1|name:Agility|target:Galvantula,player-1,1",
            "boost|mon:Galvantula,player-1,1|stat:spe|by:2",
            "move|mon:Galvantula,player-2,1|name:Recover|target:Galvantula,player-2,1",
            "split|side:1",
            "heal|mon:Galvantula,player-2,1|health:130/130",
            "heal|mon:Galvantula,player-2,1|health:100/100",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Galvantula,player-1,1|name:Electro Ball|target:Galvantula,player-2,1",
            "resisted|mon:Galvantula,player-2,1",
            "split|side:1",
            "damage|mon:Galvantula,player-2,1|health:77/130",
            "damage|mon:Galvantula,player-2,1|health:60/100",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Galvantula,player-1,1|name:Agility|target:Galvantula,player-1,1",
            "boost|mon:Galvantula,player-1,1|stat:spe|by:2",
            "move|mon:Galvantula,player-2,1|name:Recover|target:Galvantula,player-2,1",
            "split|side:1",
            "heal|mon:Galvantula,player-2,1|health:130/130",
            "heal|mon:Galvantula,player-2,1|health:100/100",
            "residual",
            "turn|turn:7",
            "continue",
            "move|mon:Galvantula,player-1,1|name:Electro Ball|target:Galvantula,player-2,1",
            "resisted|mon:Galvantula,player-2,1",
            "split|side:1",
            "damage|mon:Galvantula,player-2,1|health:59/130",
            "damage|mon:Galvantula,player-2,1|health:46/100",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn electro_ball_min_damage_slower() {
    let mut battle = make_battle(0, galvantula().unwrap(), galvantula().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: P2 Agility. P1 Pass.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    // Turn 2: P1 Electro Ball. P2 Pass.
    // P2 Speed: 256. P1 Speed: 128. Ratio: 0.5. Expect 40 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Galvantula,player-2,1|name:Agility|target:Galvantula,player-2,1",
            "boost|mon:Galvantula,player-2,1|stat:spe|by:2",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Galvantula,player-1,1|name:Electro Ball|target:Galvantula,player-2,1",
            "resisted|mon:Galvantula,player-2,1",
            "split|side:1",
            "damage|mon:Galvantula,player-2,1|health:109/130",
            "damage|mon:Galvantula,player-2,1|health:84/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
