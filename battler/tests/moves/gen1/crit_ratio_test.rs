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
fn increased_crit_ratio() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Razor Leaf"
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
    let mut battle = make_battle(&data, team.clone(), team, 881531188942077).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Razor Leaf",
            "resisted|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:97/105",
            "damage|mon:Bulbasaur,player-2,1|health:93/100",
            "move|mon:Bulbasaur,player-2,1|name:Razor Leaf",
            "resisted|mon:Bulbasaur,player-1,1",
            "crit|mon:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:91/105",
            "damage|mon:Bulbasaur,player-1,1|health:87/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn moves_can_force_crit() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Frost Breath"
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
    let mut battle = make_battle(&data, team.clone(), team, 1).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Frost Breath|target:Bulbasaur,player-2,1",
            "supereffective|mon:Bulbasaur,player-2,1",
            "crit|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:25/105",
            "damage|mon:Bulbasaur,player-2,1|health:24/100",
            "move|mon:Bulbasaur,player-2,1|name:Frost Breath|target:Bulbasaur,player-1,1",
            "supereffective|mon:Bulbasaur,player-1,1",
            "crit|mon:Bulbasaur,player-1,1",
            "split|side:0",
            "damage|mon:Bulbasaur,player-1,1|health:23/105",
            "damage|mon:Bulbasaur,player-1,1|health:22/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn crit_ignores_stat_modifiers() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bulbasaur",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": [
                        "Frost Breath",
                        "Calm Mind",
                        "Recover"
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
    let mut battle = make_battle(&data, team.clone(), team, 1).unwrap();
    assert_eq!(battle.start(), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Bulbasaur,player-1,1|name:Frost Breath|target:Bulbasaur,player-2,1",
            "supereffective|mon:Bulbasaur,player-2,1",
            "crit|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:25/105",
            "damage|mon:Bulbasaur,player-2,1|health:24/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Bulbasaur,player-2,1|name:Recover|target:Bulbasaur,player-2,1",
            "split|side:1",
            "heal|mon:Bulbasaur,player-2,1|health:78/105",
            "heal|mon:Bulbasaur,player-2,1|health:75/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Bulbasaur,player-2,1|name:Calm Mind|target:Bulbasaur,player-2,1",
            "boost|mon:Bulbasaur,player-2,1|stat:spa|by:1",
            "boost|mon:Bulbasaur,player-2,1|stat:spd|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Bulbasaur,player-2,1|name:Calm Mind|target:Bulbasaur,player-2,1",
            "boost|mon:Bulbasaur,player-2,1|stat:spa|by:1",
            "boost|mon:Bulbasaur,player-2,1|stat:spd|by:1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Bulbasaur,player-2,1|name:Calm Mind|target:Bulbasaur,player-2,1",
            "boost|mon:Bulbasaur,player-2,1|stat:spa|by:1",
            "boost|mon:Bulbasaur,player-2,1|stat:spd|by:1",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Bulbasaur,player-1,1|name:Frost Breath|target:Bulbasaur,player-2,1",
            "supereffective|mon:Bulbasaur,player-2,1",
            "crit|mon:Bulbasaur,player-2,1",
            "split|side:1",
            "damage|mon:Bulbasaur,player-2,1|health:0",
            "damage|mon:Bulbasaur,player-2,1|health:0",
            "faint|mon:Bulbasaur,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
