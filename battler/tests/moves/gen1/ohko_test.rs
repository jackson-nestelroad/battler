use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::Error,
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

fn make_battle(
    data: &dyn DataStore,
    team_1: TeamData,
    team_2: TeamData,
    seed: u64,
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
fn ohko_lower_level_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Fissure"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ivysaur",
                    "species": "Ivysaur",
                    "ability": "Overgrow",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 20
                }
            ]
        }"#,
    )
    .unwrap();

    let mut battle = make_battle(&data, team_1, team_2, 71576326561355).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Fissure|target:Ivysaur,player-2,1",
            "split|side:1",
            "damage|mon:Ivysaur,player-2,1|health:0",
            "damage|mon:Ivysaur,player-2,1|health:0",
            "ohko|mon:Ivysaur,player-2,1",
            "faint|mon:Ivysaur,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ohko_fails_for_higher_level_target() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Fissure"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 40
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();

    let mut battle = make_battle(&data, team_1, team_2, 2452345434).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Fissure|noanim",
            "immune|mon:Venusaur,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ohko_for_specific_type() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Lapras",
                    "species": "Lapras",
                    "ability": "Water Absorb",
                    "moves": [
                        "Sheer Cold"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();

    let mut battle = make_battle(&data, team_1, team_2, 1022714371015146).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lapras,player-1,1|name:Sheer Cold|target:Venusaur,player-2,1",
            "split|side:1",
            "damage|mon:Venusaur,player-2,1|health:0",
            "damage|mon:Venusaur,player-2,1|health:0",
            "ohko|mon:Venusaur,player-2,1",
            "faint|mon:Venusaur,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ohko_fails_against_specific_type() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Lapras",
                    "species": "Lapras",
                    "ability": "Water Absorb",
                    "moves": [
                        "Sheer Cold"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Lapras",
                    "species": "Lapras",
                    "ability": "Water Absorb",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();

    let mut battle = make_battle(&data, team_1, team_2, 1111110000111).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lapras,player-1,1|name:Sheer Cold|noanim",
            "immune|mon:Lapras,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
