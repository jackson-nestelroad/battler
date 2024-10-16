use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
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

fn applin() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Applin",
                    "species": "Applin",
                    "ability": "Ripen",
                    "moves": [
                        "Dragon Claw",
                        "Splash"
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
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn ripen_doubles_damage_healed_by_oran_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = applin().unwrap();
    team.members[0].item = Some("Oran Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, applin().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Applin,player-2,1|name:Dragon Claw|target:Applin,player-1,1",
            "supereffective|mon:Applin,player-1,1",
            "split|side:0",
            "damage|mon:Applin,player-1,1|health:44/100",
            "damage|mon:Applin,player-1,1|health:44/100",
            "activate|mon:Applin,player-1,1|ability:Ripen",
            "itemend|mon:Applin,player-1,1|item:Oran Berry|eat",
            "activate|mon:Applin,player-1,1|ability:Ripen",
            "split|side:0",
            "heal|mon:Applin,player-1,1|from:item:Oran Berry|health:64/100",
            "heal|mon:Applin,player-1,1|from:item:Oran Berry|health:64/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ripen_doubles_damage_healed_by_sitrus_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = applin().unwrap();
    team.members[0].item = Some("Sitrus Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, applin().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Applin,player-2,1|name:Dragon Claw|target:Applin,player-1,1",
            "supereffective|mon:Applin,player-1,1",
            "split|side:0",
            "damage|mon:Applin,player-1,1|health:44/100",
            "damage|mon:Applin,player-1,1|health:44/100",
            "activate|mon:Applin,player-1,1|ability:Ripen",
            "itemend|mon:Applin,player-1,1|item:Sitrus Berry|eat",
            "activate|mon:Applin,player-1,1|ability:Ripen",
            "split|side:0",
            "heal|mon:Applin,player-1,1|from:item:Sitrus Berry|health:94/100",
            "heal|mon:Applin,player-1,1|from:item:Sitrus Berry|health:94/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ripen_doubles_damage_reduced_by_haban_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = applin().unwrap();
    team.members[0].item = Some("Haban Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, applin().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Applin,player-2,1|name:Dragon Claw|target:Applin,player-1,1",
            "supereffective|mon:Applin,player-1,1",
            "activate|mon:Applin,player-1,1|ability:Ripen",
            "itemend|mon:Applin,player-1,1|item:Haban Berry|eat",
            "activate|mon:Applin,player-1,1|item:Haban Berry|weaken",
            "split|side:0",
            "damage|mon:Applin,player-1,1|health:86/100",
            "damage|mon:Applin,player-1,1|health:86/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ripen_doubles_stat_boost_by_kee_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = applin().unwrap();
    team.members[0].item = Some("Kee Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, applin().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Applin,player-2,1|name:Dragon Claw|target:Applin,player-1,1",
            "supereffective|mon:Applin,player-1,1",
            "split|side:0",
            "damage|mon:Applin,player-1,1|health:44/100",
            "damage|mon:Applin,player-1,1|health:44/100",
            "activate|mon:Applin,player-1,1|ability:Ripen",
            "itemend|mon:Applin,player-1,1|item:Kee Berry|eat",
            "boost|mon:Applin,player-1,1|stat:def|by:2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ripen_doubles_damage_dealt_by_jaboca_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = applin().unwrap();
    team.members[0].item = Some("Jaboca Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, applin().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Applin,player-2,1|name:Dragon Claw|target:Applin,player-1,1",
            "supereffective|mon:Applin,player-1,1",
            "split|side:0",
            "damage|mon:Applin,player-1,1|health:44/100",
            "damage|mon:Applin,player-1,1|health:44/100",
            "activate|mon:Applin,player-1,1|ability:Ripen",
            "itemend|mon:Applin,player-1,1|item:Jaboca Berry|eat",
            "split|side:1",
            "damage|mon:Applin,player-2,1|from:item:Jaboca Berry|of:Applin,player-1,1|health:76/100",
            "damage|mon:Applin,player-2,1|from:item:Jaboca Berry|of:Applin,player-1,1|health:76/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ripen_doubles_pp_restored_by_leppa_berry() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = applin().unwrap();
    team.members[0].item = Some("Leppa Berry".to_owned());
    let mut battle = make_battle(&data, 0, team, applin().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    for _ in 0..40 {
        assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
        assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    }

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Applin,player-1,1|name:Splash|target:Applin,player-1,1",
            "activate|move:Splash",
            "activate|mon:Applin,player-1,1|ability:Ripen",
            "itemend|mon:Applin,player-1,1|item:Leppa Berry|eat",
            "restorepp|mon:Applin,player-1,1|move:Splash|by:20|from:item:Leppa Berry",
            "residual",
            "turn|turn:41"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 40, &expected_logs);
}
