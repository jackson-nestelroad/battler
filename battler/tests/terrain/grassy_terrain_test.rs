use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn furfrou() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Furfrou",
                    "species": "Furfrou",
                    "ability": "No Ability",
                    "moves": [
                        "Grassy Terrain",
                        "Vine Whip",
                        "Earthquake",
                        "Hyper Voice"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn fletchling() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Fletchling",
                    "species": "Fletchling",
                    "ability": "No Ability",
                    "moves": [],
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn grassy_terrain_boosts_grass_move_power() {
    let mut battle = make_battle(0, furfrou().unwrap(), furfrou().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Vine Whip|target:Furfrou,player-2,1",
            "split|side:1",
            "damage|mon:Furfrou,player-2,1|health:108/135",
            "damage|mon:Furfrou,player-2,1|health:80/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "split|side:1",
            "heal|mon:Furfrou,player-2,1|from:move:Grassy Terrain|health:116/135",
            "heal|mon:Furfrou,player-2,1|from:move:Grassy Terrain|health:86/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Vine Whip|target:Furfrou,player-2,1",
            "split|side:1",
            "damage|mon:Furfrou,player-2,1|health:81/135",
            "damage|mon:Furfrou,player-2,1|health:60/100",
            "split|side:1",
            "heal|mon:Furfrou,player-2,1|from:move:Grassy Terrain|health:89/135",
            "heal|mon:Furfrou,player-2,1|from:move:Grassy Terrain|health:66/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn grassy_terrain_weakens_earthquake() {
    let mut battle = make_battle(0, furfrou().unwrap(), furfrou().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Earthquake",
            "split|side:1",
            "damage|mon:Furfrou,player-2,1|health:76/135",
            "damage|mon:Furfrou,player-2,1|health:57/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "split|side:1",
            "heal|mon:Furfrou,player-2,1|from:move:Grassy Terrain|health:84/135",
            "heal|mon:Furfrou,player-2,1|from:move:Grassy Terrain|health:63/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Earthquake",
            "split|side:1",
            "damage|mon:Furfrou,player-2,1|health:54/135",
            "damage|mon:Furfrou,player-2,1|health:40/100",
            "split|side:1",
            "heal|mon:Furfrou,player-2,1|from:move:Grassy Terrain|health:62/135",
            "heal|mon:Furfrou,player-2,1|from:move:Grassy Terrain|health:46/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn grassy_terrain_does_not_heal_ungrounded_mon() {
    let mut battle = make_battle(0, furfrou().unwrap(), fletchling().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Hyper Voice",
            "split|side:1",
            "damage|mon:Fletchling,player-2,1|health:6/105",
            "damage|mon:Fletchling,player-2,1|health:6/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn grassy_terrain_lasts_five_turns() {
    let mut battle = make_battle(0, furfrou().unwrap(), furfrou().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "residual",
            "turn|turn:2",
            ["time"],
            "residual",
            "turn|turn:3",
            ["time"],
            "residual",
            "turn|turn:4",
            ["time"],
            "residual",
            "turn|turn:5",
            ["time"],
            "fieldend|move:Grassy Terrain",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn grassy_terrain_lasts_eight_turns_with_terrain_extender() {
    let mut team = furfrou().unwrap();
    team.members[0].item = Some("Terrain Extender".to_owned());
    let mut battle = make_battle(0, team, furfrou().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "residual",
            "turn|turn:2",
            ["time"],
            "residual",
            "turn|turn:3",
            ["time"],
            "residual",
            "turn|turn:4",
            ["time"],
            "residual",
            "turn|turn:5",
            ["time"],
            "residual",
            "turn|turn:6",
            ["time"],
            "residual",
            "turn|turn:7",
            ["time"],
            "residual",
            "turn|turn:8",
            ["time"],
            "fieldend|move:Grassy Terrain",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
