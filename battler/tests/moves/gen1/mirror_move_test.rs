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
                    "name": "Pidgeot",
                    "species": "Pidgeot",
                    "ability": "No Ability",
                    "moves": [
                        "Mirror Move",
                        "Agility",
                        "Quick Attack",
                        "Razor Wind",
                        "Fly"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Thunder Shock"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
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
fn mirror_move_copies_targets_last_move() {
    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Fails with no last move.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Fails to copy itself.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Copy fails.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Copy succeeds.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    // Copy of the copy fails.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Two-turn move, last move is copied (not this one).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Two-turn move finishes and is copied.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Last move was Mirror Move, so the copy fails. Copied two-turn move finishes.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // The copied two-turn move cannot be copied (there is actually a big difference in the
    // battle engine).
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pidgeot,player-1,1|name:Mirror Move|noanim",
            "fail|mon:Pidgeot,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Mirror Move|noanim",
            "fail|mon:Pidgeot,player-1,1",
            "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
            "fail|mon:Pidgeot,player-2,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Agility|target:Pidgeot,player-1,1",
            "boost|mon:Pidgeot,player-1,1|stat:spe|by:2",
            "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
            "fail|mon:Pidgeot,player-2,1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pidgeot,player-2,1|name:Quick Attack|target:Pidgeot,player-1,1",
            "split|side:0",
            "damage|mon:Pidgeot,player-1,1|health:115/143",
            "damage|mon:Pidgeot,player-1,1|health:81/100",
            "move|mon:Pidgeot,player-1,1|name:Mirror Move|target:Pidgeot,player-2,1",
            "move|mon:Pidgeot,player-1,1|name:Quick Attack|target:Pidgeot,player-2,1|from:move:Mirror Move",
            "split|side:1",
            "damage|mon:Pidgeot,player-2,1|health:116/143",
            "damage|mon:Pidgeot,player-2,1|health:82/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
            "fail|mon:Pidgeot,player-2,1",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Razor Wind|noanim",
            "prepare|mon:Pidgeot,player-1,1|move:Razor Wind",
            "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
            "fail|mon:Pidgeot,player-2,1",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Razor Wind",
            "split|side:1",
            "damage|mon:Pidgeot,player-2,1|health:70/143",
            "damage|mon:Pidgeot,player-2,1|health:49/100",
            "move|mon:Pidgeot,player-2,1|name:Mirror Move|target:Pidgeot,player-1,1",
            "move|mon:Pidgeot,player-2,1|name:Razor Wind|from:move:Mirror Move|noanim",
            "prepare|mon:Pidgeot,player-2,1|move:Razor Wind",
            "residual",
            "turn|turn:8",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Mirror Move|noanim",
            "fail|mon:Pidgeot,player-1,1",
            "move|mon:Pidgeot,player-2,1|name:Razor Wind",
            "split|side:0",
            "damage|mon:Pidgeot,player-1,1|health:66/143",
            "damage|mon:Pidgeot,player-1,1|health:47/100",
            "residual",
            "turn|turn:9",
            ["time"],
            "move|mon:Pidgeot,player-2,1|name:Mirror Move|noanim",
            "fail|mon:Pidgeot,player-2,1",
            "residual",
            "turn|turn:10"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mirror_move_locks_target_like_source_move() {
    let mut battle = make_battle(BattleType::Doubles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 4,2;pass"),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pidgeot,player-2,1|name:Fly|noanim",
            "prepare|mon:Pidgeot,player-2,1|move:Fly",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pidgeot,player-2,1|name:Fly|target:Pikachu,player-1,2",
            "resisted|mon:Pikachu,player-1,2",
            "split|side:0",
            "damage|mon:Pikachu,player-1,2|health:41/95",
            "damage|mon:Pikachu,player-1,2|health:44/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Mirror Move|target:Pidgeot,player-2,1",
            "move|mon:Pidgeot,player-1,1|name:Fly|from:move:Mirror Move|noanim",
            "prepare|mon:Pidgeot,player-1,1|move:Fly",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pidgeot,player-1,1|name:Fly|target:Pidgeot,player-2,1",
            "split|side:1",
            "damage|mon:Pidgeot,player-2,1|health:85/143",
            "damage|mon:Pidgeot,player-2,1|health:60/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
