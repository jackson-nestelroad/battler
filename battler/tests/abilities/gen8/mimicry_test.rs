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
                    "name": "Stunfisk",
                    "species": "Stunfisk-Galar",
                    "ability": "Mimicry",
                    "moves": [
                        "Electric Terrain",
                        "Grassy Terrain",
                        "Psychic Terrain",
                        "Steel Roller",
                        "Shadow Claw",
                        "Forest's Curse",
                        "Frost Breath"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "Imposter",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Rillaboom",
                    "species": "Rillaboom",
                    "ability": "Mimicry",
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
fn mimicry_changes_type_to_match_terrain() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Stunfisk,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "typechange|mon:Stunfisk,player-1,1|types:Electric|from:ability:Mimicry",
            "typechange|mon:Stunfisk,player-2,1|types:Electric|from:ability:Mimicry",
            "move|mon:Stunfisk,player-2,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "typechange|mon:Stunfisk,player-1,1|types:Grass|from:ability:Mimicry",
            "typechange|mon:Stunfisk,player-2,1|types:Grass|from:ability:Mimicry",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Stunfisk,player-1,1|name:Psychic Terrain",
            "fieldstart|move:Psychic Terrain",
            "typechange|mon:Stunfisk,player-1,1|types:Psychic|from:ability:Mimicry",
            "typechange|mon:Stunfisk,player-2,1|types:Psychic|from:ability:Mimicry",
            "move|mon:Stunfisk,player-2,1|name:Steel Roller|target:Stunfisk,player-1,1",
            "split|side:0",
            "damage|mon:Stunfisk,player-1,1|health:240/328",
            "damage|mon:Stunfisk,player-1,1|health:74/100",
            "fieldend|move:Psychic Terrain",
            "resettypechange|mon:Stunfisk,player-1,1|from:ability:Mimicry",
            "resettypechange|mon:Stunfisk,player-2,1|from:ability:Mimicry",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mimicry_resets_type_to_base_species_types_on_reset() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Ditto"],
            ["switch", "player-2", "Ditto"],
            "resettypechange|mon:Ditto,player-2,1|from:ability:Mimicry",
            "transform|mon:Ditto,player-2,1|into:Stunfisk,player-1,1|species:Stunfisk-Galar|from:ability:Imposter",
            "move|mon:Stunfisk,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "typechange|mon:Stunfisk,player-1,1|types:Electric|from:ability:Mimicry",
            "typechange|mon:Ditto,player-2,1|types:Electric|from:ability:Mimicry",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Stunfisk,player-1,1|name:Steel Roller|target:Ditto,player-2,1",
            "resisted|mon:Ditto,player-2,1",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:162/206",
            "damage|mon:Ditto,player-2,1|health:79/100",
            "fieldend|move:Electric Terrain",
            "resettypechange|mon:Stunfisk,player-1,1|from:ability:Mimicry",
            "resettypechange|mon:Ditto,player-2,1|from:ability:Mimicry",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Stunfisk,player-1,1|name:Shadow Claw|noanim",
            "immune|mon:Ditto,player-2,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mimicry_removes_added_type_only_if_duplicated() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 6"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 6"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Stunfisk,player-1,1|name:Forest's Curse|target:Stunfisk,player-2,1",
            "addedtype|mon:Stunfisk,player-2,1|type:Grass",
            "move|mon:Stunfisk,player-2,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "typechange|mon:Stunfisk,player-1,1|types:Electric|from:ability:Mimicry",
            "typechange|mon:Stunfisk,player-2,1|types:Electric|from:ability:Mimicry",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Stunfisk,player-1,1|name:Frost Breath|target:Stunfisk,player-2,1",
            "supereffective|mon:Stunfisk,player-2,1",
            "crit|mon:Stunfisk,player-2,1",
            "split|side:1",
            "damage|mon:Stunfisk,player-2,1|health:210/328",
            "damage|mon:Stunfisk,player-2,1|health:65/100",
            "move|mon:Stunfisk,player-2,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "typechange|mon:Stunfisk,player-1,1|types:Grass|from:ability:Mimicry",
            "typechange|mon:Stunfisk,player-2,1|types:Grass|from:ability:Mimicry",
            "split|side:1",
            "heal|mon:Stunfisk,player-2,1|from:move:Grassy Terrain|health:230/328",
            "heal|mon:Stunfisk,player-2,1|from:move:Grassy Terrain|health:71/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Stunfisk,player-1,1|name:Steel Roller|target:Stunfisk,player-2,1",
            "split|side:1",
            "damage|mon:Stunfisk,player-2,1|health:149/328",
            "damage|mon:Stunfisk,player-2,1|health:46/100",
            "fieldend|move:Grassy Terrain",
            "resettypechange|mon:Stunfisk,player-1,1|from:ability:Mimicry",
            "resettypechange|mon:Stunfisk,player-2,1|from:ability:Mimicry",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Stunfisk,player-1,1|name:Frost Breath|target:Stunfisk,player-2,1",
            "crit|mon:Stunfisk,player-2,1",
            "split|side:1",
            "damage|mon:Stunfisk,player-2,1|health:92/328",
            "damage|mon:Stunfisk,player-2,1|health:29/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mimicry_does_not_activate_if_type_does_not_change() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Rillaboom"],
            ["switch", "player-1", "Rillaboom"],
            "move|mon:Stunfisk,player-2,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "typechange|mon:Stunfisk,player-2,1|types:Grass|from:ability:Mimicry",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Stunfisk,player-2,1|name:Steel Roller|target:Rillaboom,player-1,1",
            "split|side:0",
            "damage|mon:Rillaboom,player-1,1|health:213/310",
            "damage|mon:Rillaboom,player-1,1|health:69/100",
            "fieldend|move:Grassy Terrain",
            "resettypechange|mon:Stunfisk,player-2,1|from:ability:Mimicry",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
