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
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "No Ability",
                    "moves": [
                        "Dragon Cheer",
                        "Transform"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Raging Bolt",
                    "species": "Raging Bolt",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Focus Energy"
                    ],
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
fn dragon_cheer_boosts_ally_critical_hit_ratio() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-1,1|name:Dragon Cheer|target:Raging Bolt,player-1,2",
            "start|mon:Raging Bolt,player-1,2|move:Dragon Cheer",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Raging Bolt,player-1,2|name:Tackle|target:Ditto,player-2,1",
            "crit|mon:Ditto,player-2,1",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:131/206",
            "damage|mon:Ditto,player-2,1|health:64/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dragon_cheer_copied_by_transform() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-1,1|name:Dragon Cheer|target:Raging Bolt,player-1,2",
            "start|mon:Raging Bolt,player-1,2|move:Dragon Cheer",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Ditto,player-2,1|name:Transform|target:Raging Bolt,player-1,2",
            "transform|mon:Ditto,player-2,1|into:Raging Bolt,player-1,2|species:Raging Bolt",
            "start|mon:Ditto,player-2,1|move:Dragon Cheer|silent",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Ditto,player-2,1|name:Tackle|target:Ditto,player-1,1",
            "crit|mon:Ditto,player-1,1",
            "split|side:0",
            "damage|mon:Ditto,player-1,1|health:131/206",
            "damage|mon:Ditto,player-1,1|health:64/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn focus_energy_cannot_stack_with_dragon_cheer() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;move 1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Raging Bolt,player-1,2|name:Focus Energy|target:Raging Bolt,player-1,2",
            "start|mon:Raging Bolt,player-1,2|move:Focus Energy",
            "move|mon:Ditto,player-1,1|name:Dragon Cheer|noanim",
            "fail|mon:Ditto,player-1,1",
            "move|mon:Ditto,player-2,1|name:Dragon Cheer|target:Raging Bolt,player-2,2",
            "start|mon:Raging Bolt,player-2,2|move:Dragon Cheer",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Raging Bolt,player-2,2|name:Focus Energy|noanim",
            "fail|mon:Raging Bolt,player-2,2",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
