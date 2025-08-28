use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
};

fn giratina() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Giratina",
                    "species": "Giratina-Origin",
                    "ability": "No Ability",
                    "item": "Griseous Orb",
                    "moves": [
                        "Fling"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn empoleon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Empoleon",
                    "species": "Empoleon",
                    "ability": "No Ability",
                    "moves": [
                        "Thief"
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
) -> Result<PublicCoreBattle<'_>> {
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
        .build(data)
}

#[test]
fn griseous_orb_cannot_be_taken_from_giratina() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, giratina().unwrap(), empoleon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Empoleon,player-2,1|name:Thief|target:Giratina,player-1,1",
            "supereffective|mon:Giratina,player-1,1",
            "split|side:0",
            "damage|mon:Giratina,player-1,1|health:164/210",
            "damage|mon:Giratina,player-1,1|health:79/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn griseous_orb_can_be_taken_from_non_giratina() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = empoleon().unwrap();
    team.members[0].item = Some("Griseous Orb".to_owned());
    let mut battle = make_battle(&data, 0, team, empoleon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Empoleon,player-2,1|name:Thief|target:Empoleon,player-1,1",
            "split|side:0",
            "damage|mon:Empoleon,player-1,1|health:118/144",
            "damage|mon:Empoleon,player-1,1|health:82/100",
            "itemend|mon:Empoleon,player-1,1|item:Griseous Orb|silent|from:move:Thief|of:Empoleon,player-2,1",
            "item|mon:Empoleon,player-2,1|item:Griseous Orb|from:move:Thief|of:Empoleon,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn giratina_transforms_into_origin_forme_if_incorrect() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = giratina().unwrap();
    team.members[0].species = "Giratina".to_owned();
    let mut battle = make_battle(&data, 0, team, giratina().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1"],
            ["switch", "player-1"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "formechange|mon:Giratina,player-1,1|species:Giratina-Origin|from:species:Giratina",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn giratina_transforms_into_altered_forme_if_incorrect() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = giratina().unwrap();
    team.members[0].item = None;
    let mut battle = make_battle(&data, 0, team, giratina().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "species:Giratina-Origin"],
            ["switch", "player-1", "species:Giratina-Origin"],
            "split|side:1",
            ["switch", "player-2"],
            ["switch", "player-2"],
            "formechange|mon:Giratina,player-1,1|species:Giratina|from:species:Giratina-Origin",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
