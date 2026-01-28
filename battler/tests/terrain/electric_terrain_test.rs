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
                        "Electric Terrain",
                        "Thunderbolt",
                        "Yawn",
                        "Sleep Powder"
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn electric_terrain_boosts_electric_move_power() {
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
            "move|mon:Furfrou,player-1,1|name:Thunderbolt|target:Furfrou,player-2,1",
            "split|side:1",
            "damage|mon:Furfrou,player-2,1|health:104/135",
            "damage|mon:Furfrou,player-2,1|health:78/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Furfrou,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Furfrou,player-1,1|name:Thunderbolt|target:Furfrou,player-2,1",
            "split|side:1",
            "damage|mon:Furfrou,player-2,1|health:65/135",
            "damage|mon:Furfrou,player-2,1|health:49/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn electric_terrain_fails_yawn() {
    let mut battle = make_battle(0, furfrou().unwrap(), furfrou().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Furfrou,player-1,1|name:Yawn|noanim",
            "activate|mon:Furfrou,player-2,1|move:Electric Terrain",
            "fail|mon:Furfrou,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn electric_terrain_prevents_sleep() {
    let mut battle = make_battle(0, furfrou().unwrap(), furfrou().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Furfrou,player-1,1|name:Sleep Powder|noanim",
            "activate|mon:Furfrou,player-2,1|move:Electric Terrain",
            "fail|mon:Furfrou,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn electric_terrain_lasts_five_turns() {
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
            "move|mon:Furfrou,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "residual",
            "turn|turn:2",
            "continue",
            "residual",
            "turn|turn:3",
            "continue",
            "residual",
            "turn|turn:4",
            "continue",
            "residual",
            "turn|turn:5",
            "continue",
            "fieldend|move:Electric Terrain",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn electric_terrain_lasts_eight_turns_with_terrain_extender() {
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
            "move|mon:Furfrou,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "residual",
            "turn|turn:2",
            "continue",
            "residual",
            "turn|turn:3",
            "continue",
            "residual",
            "turn|turn:4",
            "continue",
            "residual",
            "turn|turn:5",
            "continue",
            "residual",
            "turn|turn:6",
            "continue",
            "residual",
            "turn|turn:7",
            "continue",
            "residual",
            "turn|turn:8",
            "continue",
            "fieldend|move:Electric Terrain",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
