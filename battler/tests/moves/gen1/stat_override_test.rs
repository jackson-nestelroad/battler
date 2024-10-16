use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::Error,
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
        .with_seed(0)
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
fn override_defensive_stat() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Psyshock",
                        "Defense Curl"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team.clone(), team).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Psyshock|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:52/95",
            "damage|mon:Pikachu,player-2,1|health:55/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-2,1|name:Defense Curl|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:def|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Psyshock|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:25/95",
            "damage|mon:Pikachu,player-2,1|health:27/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn override_damage_calculation_mon() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Slaking",
                    "species": "Slaking",
                    "ability": "No Ability",
                    "moves": [
                        "Foul Play",
                        "Howl"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team.clone(), team).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Slaking,player-1,1|name:Foul Play|target:Slaking,player-2,1",
            "split|side:1",
            "damage|mon:Slaking,player-2,1|health:146/210",
            "damage|mon:Slaking,player-2,1|health:70/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Slaking,player-2,1|name:Howl",
            "boost|mon:Slaking,player-2,1|stat:atk|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Slaking,player-1,1|name:Foul Play|target:Slaking,player-2,1",
            "split|side:1",
            "damage|mon:Slaking,player-2,1|health:56/210",
            "damage|mon:Slaking,player-2,1|health:27/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn override_offensive_stat() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Slaking",
                    "species": "Slaking",
                    "ability": "No Ability",
                    "moves": [
                        "Body Press",
                        "Defense Curl"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_battle(&data, team.clone(), team).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Slaking,player-1,1|name:Body Press|target:Slaking,player-2,1",
            "supereffective|mon:Slaking,player-2,1",
            "split|side:1",
            "damage|mon:Slaking,player-2,1|health:140/210",
            "damage|mon:Slaking,player-2,1|health:67/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Slaking,player-1,1|name:Defense Curl|target:Slaking,player-1,1",
            "boost|mon:Slaking,player-1,1|stat:def|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Slaking,player-1,1|name:Body Press|target:Slaking,player-2,1",
            "supereffective|mon:Slaking,player-2,1",
            "split|side:1",
            "damage|mon:Slaking,player-2,1|health:44/210",
            "damage|mon:Slaking,player-2,1|health:21/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
