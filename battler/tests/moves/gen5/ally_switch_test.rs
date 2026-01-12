use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn gothitelle() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Foo",
                    "species": "Gothitelle",
                    "ability": "No Ability",
                    "moves": [
                        "Ally Switch",
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Bar",
                    "species": "Gothitelle",
                    "ability": "No Ability",
                    "moves": [
                        "Ally Switch",
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Baz",
                    "species": "Gothitelle",
                    "ability": "No Ability",
                    "moves": [
                        "Ally Switch",
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(
    battle_type: BattleType,
    seed: u64,
    team: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team.clone())
        .with_team("player-2", team)
        .build(static_local_data_store())
}

#[test]
fn ally_switch_swaps_positions() {
    let team = gothitelle();
    let mut battle = make_battle(BattleType::Doubles, 0, team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0; pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass; pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1; move 1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass; pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Foo,player-1,1|name:Ally Switch|target:Foo,player-1,1",
            "swap|mon:Foo,player-1,1|position:2|from:move:Ally Switch",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Bar,player-1,1|name:Splash|target:Bar,player-1,1",
            "activate|move:Splash",
            "move|mon:Foo,player-1,2|name:Splash|target:Foo,player-1,2",
            "activate|move:Splash",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ally_switch_fails_single_battle() {
    let team = gothitelle();
    let mut battle = make_battle(BattleType::Singles, 0, team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Foo,player-1,1|name:Ally Switch|noanim",
            "fail|mon:Foo,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ally_switch_swaps_positions_triples() {
    let team = gothitelle();
    let mut battle = make_battle(BattleType::Triples, 0, team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0; pass; pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass; pass; pass"),
        Ok(())
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1; pass; move 1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass; pass; pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Foo,player-1,1|name:Ally Switch|target:Foo,player-1,1",
            "swap|mon:Foo,player-1,1|position:3|from:move:Ally Switch",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Baz,player-1,1|name:Splash|target:Baz,player-1,1",
            "activate|move:Splash",
            "move|mon:Foo,player-1,3|name:Splash|target:Foo,player-1,3",
            "activate|move:Splash",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ally_switch_fails_middle_triples() {
    let team = gothitelle();
    let mut battle = make_battle(BattleType::Triples, 0, team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass; move 0; pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass; pass; pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bar,player-1,2|name:Ally Switch|noanim",
            "fail|mon:Bar,player-1,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ally_switch_fails_partner_fainted() {
    let mut team = gothitelle();
    team.members[1].persistent_battle_data.hp = Some(0);
    let mut battle = make_battle(BattleType::Doubles, 0, team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0; pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass; pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Foo,player-1,1|name:Ally Switch|target:Foo,player-1,1",
            "swap|mon:Foo,player-1,1|position:2|from:move:Ally Switch",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ally_switch_consecutive_fails() {
    let team = gothitelle();
    let mut battle = make_battle(BattleType::Doubles, 0, team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0; move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass; pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0; move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass; pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Foo,player-1,1|name:Ally Switch|target:Foo,player-1,1",
            "swap|mon:Foo,player-1,1|position:2|from:move:Ally Switch",
            "move|mon:Bar,player-1,1|name:Ally Switch|target:Bar,player-1,1",
            "swap|mon:Bar,player-1,1|position:2|from:move:Ally Switch",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Foo,player-1,1|name:Ally Switch|target:Foo,player-1,1",
            "swap|mon:Foo,player-1,1|position:2|from:move:Ally Switch",
            "move|mon:Bar,player-1,1|name:Ally Switch|noanim",
            "fail|mon:Bar,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
