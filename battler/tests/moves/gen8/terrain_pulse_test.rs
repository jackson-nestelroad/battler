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
                    "name": "Arboliva",
                    "species": "Arboliva",
                    "ability": "No Ability",
                    "moves": [
                        "Terrain Pulse",
                        "Electric Terrain",
                        "Grassy Terrain",
                        "Psychic Terrain",
                        "Misty Terrain",
                        "Telekinesis"
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
fn terrain_pulse_changes_type_based_on_effective_terrain() {
    let mut team_2 = team().unwrap();
    team_2.members[0].level = 100;
    team_2.members[0].ability = "Color Change".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Arboliva,player-1,1|name:Terrain Pulse|target:Arboliva,player-2,1",
            "split|side:1",
            "damage|mon:Arboliva,player-2,1|health:247/266",
            "damage|mon:Arboliva,player-2,1|health:93/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Arboliva,player-2,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "move|mon:Arboliva,player-1,1|name:Terrain Pulse|target:Arboliva,player-2,1",
            "resisted|mon:Arboliva,player-2,1",
            "split|side:1",
            "damage|mon:Arboliva,player-2,1|health:239/266",
            "damage|mon:Arboliva,player-2,1|health:90/100",
            "typechange|mon:Arboliva,player-2,1|types:Electric",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Arboliva,player-2,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "move|mon:Arboliva,player-1,1|name:Terrain Pulse|target:Arboliva,player-2,1",
            "split|side:1",
            "damage|mon:Arboliva,player-2,1|health:217/266",
            "damage|mon:Arboliva,player-2,1|health:82/100",
            "typechange|mon:Arboliva,player-2,1|types:Grass",
            "split|side:1",
            "heal|mon:Arboliva,player-2,1|from:move:Grassy Terrain|health:233/266",
            "heal|mon:Arboliva,player-2,1|from:move:Grassy Terrain|health:88/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Arboliva,player-2,1|name:Psychic Terrain",
            "fieldstart|move:Psychic Terrain",
            "move|mon:Arboliva,player-1,1|name:Terrain Pulse|target:Arboliva,player-2,1",
            "split|side:1",
            "damage|mon:Arboliva,player-2,1|health:217/266",
            "damage|mon:Arboliva,player-2,1|health:82/100",
            "typechange|mon:Arboliva,player-2,1|types:Psychic",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Arboliva,player-2,1|name:Misty Terrain",
            "fieldstart|move:Misty Terrain",
            "move|mon:Arboliva,player-1,1|name:Terrain Pulse|target:Arboliva,player-2,1",
            "split|side:1",
            "damage|mon:Arboliva,player-2,1|health:205/266",
            "damage|mon:Arboliva,player-2,1|health:78/100",
            "typechange|mon:Arboliva,player-2,1|types:Fairy",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Arboliva,player-2,1|name:Telekinesis|target:Arboliva,player-1,1",
            "start|mon:Arboliva,player-1,1|move:Telekinesis",
            "move|mon:Arboliva,player-1,1|name:Terrain Pulse|target:Arboliva,player-2,1",
            "split|side:1",
            "damage|mon:Arboliva,player-2,1|health:186/266",
            "damage|mon:Arboliva,player-2,1|health:70/100",
            "typechange|mon:Arboliva,player-2,1|types:Normal",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
