use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn furfrou() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Furfrou",
                    "species": "Furfrou",
                    "ability": "No Ability",
                    "moves": [
                        "Psychic Terrain",
                        "Psychic",
                        "Tackle",
                        "Quick Attack"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
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
        .build(data)
}

#[test]
fn psychic_terrain_boosts_psychic_move_power() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, furfrou().unwrap(), furfrou().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Psychic|target:Furfrou,player-2,1",
            "split|side:1",
            "damage|mon:Furfrou,player-2,1|health:104/135",
            "damage|mon:Furfrou,player-2,1|health:78/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Psychic Terrain",
            "fieldstart|move:Psychic Terrain",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Psychic|target:Furfrou,player-2,1",
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
fn psychic_terrain_fails_priority_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, furfrou().unwrap(), furfrou().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Psychic Terrain",
            "fieldstart|move:Psychic Terrain",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Tackle|target:Furfrou,player-2,1",
            "split|side:1",
            "damage|mon:Furfrou,player-2,1|health:98/135",
            "damage|mon:Furfrou,player-2,1|health:73/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Furfrou,player-1,1|name:Quick Attack|noanim",
            "activate|mon:Furfrou,player-2,1|move:Psychic Terrain",
            "fail|mon:Furfrou,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn psychic_terrain_lasts_five_turns() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, furfrou().unwrap(), furfrou().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Psychic Terrain",
            "fieldstart|move:Psychic Terrain",
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
            "fieldend|move:Psychic Terrain",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn psychic_terrain_lasts_eight_turns_with_terrain_extender() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = furfrou().unwrap();
    team.members[0].item = Some("Terrain Extender".to_owned());
    let mut battle = make_battle(&data, 0, team, furfrou().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Furfrou,player-1,1|name:Psychic Terrain",
            "fieldstart|move:Psychic Terrain",
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
            "fieldend|move:Psychic Terrain",
            "residual",
            "turn|turn:9"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
