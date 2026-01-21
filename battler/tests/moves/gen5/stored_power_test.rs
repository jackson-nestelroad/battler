use anyhow::Result;
use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    teams::TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};
use serde_json;

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Sigilyph",
                    "species": "Sigilyph",
                    "moves": ["Stored Power", "Cosmic Power", "Recover", "Screech"],
                    "level": 50,
                    "nature": "Hardy",
                    "ability": "No Ability"
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(0)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_pass_allowed(true)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn stored_power_damage_calculation() {
    let mut battle = make_battle(BattleType::Singles, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: No boosts. 20 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Cosmic Power. +1 Def, +1 SpD.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    // Turn 3: 2 boosts. 60 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    // Turn 4: Screech on Player 1.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3,1"), Ok(()));

    // Turn 5: 1 positive boost. 40 BP.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Sigilyph,player-1,1|name:Stored Power|target:Sigilyph,player-2,1",
            "resisted|mon:Sigilyph,player-2,1",
            "split|side:1",
            "damage|mon:Sigilyph,player-2,1|health:123/132",
            "damage|mon:Sigilyph,player-2,1|health:94/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Sigilyph,player-1,1|name:Cosmic Power|target:Sigilyph,player-1,1",
            "boost|mon:Sigilyph,player-1,1|stat:def|by:1",
            "boost|mon:Sigilyph,player-1,1|stat:spd|by:1",
            "move|mon:Sigilyph,player-2,1|name:Recover|target:Sigilyph,player-2,1",
            "split|side:1",
            "heal|mon:Sigilyph,player-2,1|health:132/132",
            "heal|mon:Sigilyph,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Sigilyph,player-1,1|name:Stored Power|target:Sigilyph,player-2,1",
            "resisted|mon:Sigilyph,player-2,1",
            "split|side:1",
            "damage|mon:Sigilyph,player-2,1|health:109/132",
            "damage|mon:Sigilyph,player-2,1|health:83/100",
            "move|mon:Sigilyph,player-2,1|name:Recover|target:Sigilyph,player-2,1",
            "split|side:1",
            "heal|mon:Sigilyph,player-2,1|health:132/132",
            "heal|mon:Sigilyph,player-2,1|health:100/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Sigilyph,player-2,1|name:Screech|target:Sigilyph,player-1,1",
            "unboost|mon:Sigilyph,player-1,1|stat:def|by:2",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Sigilyph,player-1,1|name:Stored Power|target:Sigilyph,player-2,1",
            "resisted|mon:Sigilyph,player-2,1",
            "split|side:1",
            "damage|mon:Sigilyph,player-2,1|health:115/132",
            "damage|mon:Sigilyph,player-2,1|health:88/100",
            "move|mon:Sigilyph,player-2,1|name:Recover|target:Sigilyph,player-2,1",
            "split|side:1",
            "heal|mon:Sigilyph,player-2,1|health:132/132",
            "heal|mon:Sigilyph,player-2,1|health:100/100",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
