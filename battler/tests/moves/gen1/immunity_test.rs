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
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
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
fn status_moves_ignore_immunity() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Sand Attack"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
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
                    "name": "Pidgey",
                    "species": "Pidgey",
                    "ability": "Big Pecks",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team_1, team_2).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Sand Attack|target:Pidgey,player-2,1",
            "unboost|mon:Pidgey,player-2,1|stat:acc|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn move_can_bypass_default_immunity_behavior() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team_1 = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Thunder Wave"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
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
                    "name": "Sandshrew",
                    "species": "Sandshrew",
                    "ability": "Sand Veil",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team_1, team_2).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Thunder Wave|noanim",
            "immune|mon:Sandshrew,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
