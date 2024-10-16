use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        FieldEnvironment,
        PublicCoreBattle,
    },
    error::{
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

fn ludicolo() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ludicolo",
                    "species": "Ludicolo",
                    "ability": "No Ability",
                    "moves": [
                        "Nature Power",
                        "Electric Terrain"
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
    environment: Option<FieldEnvironment>,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_field_environment(environment.unwrap_or_default())
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn nature_power_uses_tri_attack_by_default() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, None, ludicolo().unwrap(), ludicolo().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ludicolo,player-1,1|name:Nature Power|target:Ludicolo,player-2,1",
            "move|mon:Ludicolo,player-1,1|name:Tri Attack|target:Ludicolo,player-2,1",
            "split|side:1",
            "damage|mon:Ludicolo,player-2,1|health:108/140",
            "damage|mon:Ludicolo,player-2,1|health:78/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn electric_terrain_uses_thunderbolt() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, None, ludicolo().unwrap(), ludicolo().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ludicolo,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "move|mon:Ludicolo,player-2,1|name:Nature Power|target:Ludicolo,player-1,1",
            "move|mon:Ludicolo,player-2,1|name:Thunderbolt|target:Ludicolo,player-1,1",
            "split|side:0",
            "damage|mon:Ludicolo,player-1,1|health:94/140",
            "damage|mon:Ludicolo,player-1,1|health:68/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn field_environment_changes_nature_power() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        Some(FieldEnvironment::Ice),
        ludicolo().unwrap(),
        ludicolo().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ludicolo,player-1,1|name:Nature Power|target:Ludicolo,player-2,1",
            "move|mon:Ludicolo,player-1,1|name:Ice Beam|target:Ludicolo,player-2,1",
            "split|side:1",
            "damage|mon:Ludicolo,player-2,1|health:105/140",
            "damage|mon:Ludicolo,player-2,1|health:75/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
