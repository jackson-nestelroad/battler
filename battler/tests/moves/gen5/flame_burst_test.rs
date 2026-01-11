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

fn make_battle(
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>, anyhow::Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

fn pansear() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pansear",
                    "species": "Pansear",
                    "ability": "No Ability",
                    "moves": ["Flame Burst", "Substitute", "Protect"],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Simisear",
                    "species": "Simisear",
                    "ability": "No Ability",
                    "moves": ["Flame Burst"],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

#[test]
fn flame_burst_damages_adjacent_mons() {
    let mut battle = make_battle(pansear(), pansear()).unwrap();

    // Turn 1: Pansear hits Pansear. Simisear (adjacent ally of target) takes damage.
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    // Turn 2: Pansear hits Simisear (ally). Pansear (adjacent ally of target) takes damage.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Flame Burst|target:Pansear,player-2,1",
            "resisted|mon:Pansear,player-2,1",
            "split|side:1",
            "damage|mon:Pansear,player-2,1|health:86/110",
            "damage|mon:Pansear,player-2,1|health:79/100",
            "split|side:1",
            "damage|mon:Simisear,player-2,2|from:move:Flame Burst|of:Pansear,player-1,1|health:127/135",
            "damage|mon:Simisear,player-2,2|from:move:Flame Burst|of:Pansear,player-1,1|health:95/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pansear,player-1,1|name:Flame Burst|target:Simisear,player-1,2",
            "resisted|mon:Simisear,player-1,2",
            "split|side:0",
            "damage|mon:Simisear,player-1,2|health:117/135",
            "damage|mon:Simisear,player-1,2|health:87/100",
            "split|side:0",
            "damage|mon:Pansear,player-1,1|from:move:Flame Burst|health:104/110",
            "damage|mon:Pansear,player-1,1|from:move:Flame Burst|health:95/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flame_burst_triggers_on_substitute() {
    let mut battle = make_battle(pansear(), pansear()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Pansear (P2) sets up Substitute. Pansear (P1) passes.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));

    // Turn 2: Pansear P1 attacks Pansear P2. Splash damage should occur.
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-2,1|name:Substitute|target:Pansear,player-2,1",
            "start|mon:Pansear,player-2,1|move:Substitute",
            "split|side:1",
            "damage|mon:Pansear,player-2,1|health:83/110",
            "damage|mon:Pansear,player-2,1|health:76/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pansear,player-1,1|name:Flame Burst|target:Pansear,player-2,1",
            "resisted|mon:Pansear,player-2,1",
            "activate|mon:Pansear,player-2,1|move:Substitute|damage",
            "split|side:1",
            "damage|mon:Simisear,player-2,2|from:move:Flame Burst|of:Pansear,player-1,1|health:127/135",
            "damage|mon:Simisear,player-2,2|from:move:Flame Burst|of:Pansear,player-1,1|health:95/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flame_burst_triggers_on_ko() {
    let mut pansear_2 = pansear();
    // Reduce HP for KO.
    pansear_2.members[0].level = 1;
    let mut battle = make_battle(pansear(), pansear_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Flame Burst|target:Pansear,player-2,1",
            "resisted|mon:Pansear,player-2,1",
            "split|side:1",
            "damage|mon:Pansear,player-2,1|health:0",
            "damage|mon:Pansear,player-2,1|health:0",
            "split|side:1",
            "damage|mon:Simisear,player-2,2|from:move:Flame Burst|of:Pansear,player-1,1|health:127/135",
            "damage|mon:Simisear,player-2,2|from:move:Flame Burst|of:Pansear,player-1,1|health:95/100",
            "faint|mon:Pansear,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flame_burst_blocked_by_protect() {
    let mut battle = make_battle(pansear(), pansear()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2;pass"), Ok(()));

    // Protect prevents the move, so no splash damage should occur.
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-2,1|name:Protect|target:Pansear,player-2,1",
            "singleturn|mon:Pansear,player-2,1|move:Protect",
            "move|mon:Pansear,player-1,1|name:Flame Burst|target:Pansear,player-2,1",
            "activate|mon:Pansear,player-2,1|move:Protect",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flame_burst_blocked_by_magic_guard() {
    let mut pansear_2 = pansear();
    pansear_2.members[1].ability = "Magic Guard".to_owned();
    let mut battle = make_battle(pansear(), pansear_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Flame Burst|target:Pansear,player-2,1",
            "resisted|mon:Pansear,player-2,1",
            "split|side:1",
            "damage|mon:Pansear,player-2,1|health:86/110",
            "damage|mon:Pansear,player-2,1|health:79/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn flame_burst_ignores_flash_fire() {
    let mut pansear_2 = pansear();
    pansear_2.members[1].ability = "Flash Fire".to_owned();
    let mut battle = make_battle(pansear(), pansear_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pansear,player-1,1|name:Flame Burst|target:Pansear,player-2,1",
            "resisted|mon:Pansear,player-2,1",
            "split|side:1",
            "damage|mon:Pansear,player-2,1|health:86/110",
            "damage|mon:Pansear,player-2,1|health:79/100",
            "split|side:1",
            "damage|mon:Simisear,player-2,2|from:move:Flame Burst|of:Pansear,player-1,1|health:127/135",
            "damage|mon:Simisear,player-2,2|from:move:Flame Burst|of:Pansear,player-1,1|health:95/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
