use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    static_local_data_store,
};

fn make_singles_battle(
    team_1: TeamData,
    team_2: TeamData,
    seed: u64,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

fn make_doubles_battle(
    team_1: TeamData,
    team_2: TeamData,
    seed: u64,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn boost_stops_at_max_6() {
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Double Team"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_singles_battle(team.clone(), team, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Double Team|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:1",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Double Team|noanim",
            "boost|mon:Pikachu,player-1,1|stat:eva|by:0",
            "fail|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn drop_stops_at_max_6() {
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
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
    let mut battle = make_singles_battle(team.clone(), team, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Sand Attack|target:Pikachu,player-2,1",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:1",
            "residual",
            "turn|turn:7",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Sand Attack|noanim",
            "unboost|mon:Pikachu,player-2,1|stat:acc|by:0",
            "fail|mon:Pikachu,player-1,1",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn boosts_and_drops_cancel_out() {
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Agility",
                        "Cotton Spore"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_singles_battle(team.clone(), team, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "move|mon:Pikachu,player-2,1|name:Cotton Spore",
            "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "move|mon:Pikachu,player-2,1|name:Cotton Spore",
            "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "move|mon:Pikachu,player-2,1|name:Cotton Spore",
            "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "move|mon:Pikachu,player-2,1|name:Cotton Spore",
            "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "move|mon:Pikachu,player-2,1|name:Cotton Spore",
            "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "move|mon:Pikachu,player-2,1|name:Cotton Spore",
            "unboost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn multi_stat_boost() {
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Growth"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_singles_battle(team.clone(), team, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Growth|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:atk|by:1",
            "boost|mon:Pikachu,player-1,1|stat:spa|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn raise_all_stats_at_once() {
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Ancient Power"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_singles_battle(team.clone(), team, 0).unwrap();

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(4, 0)]);

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Ancient Power|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:68/95",
            "damage|mon:Pikachu,player-2,1|health:72/100",
            "boost|mon:Pikachu,player-1,1|stat:atk|by:1",
            "boost|mon:Pikachu,player-1,1|stat:def|by:1",
            "boost|mon:Pikachu,player-1,1|stat:spa|by:1",
            "boost|mon:Pikachu,player-1,1|stat:spd|by:1",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:1",
            "move|mon:Pikachu,player-2,1|name:Ancient Power|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:78/95",
            "damage|mon:Pikachu,player-1,1|health:83/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn drop_stats_of_all_targets() {
    let team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Tail Whip"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Tail Whip"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let mut battle = make_doubles_battle(team.clone(), team, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-1,1|name:Tail Whip|spread:Pikachu,player-2,1;Pikachu,player-2,2",
            "unboost|mon:Pikachu,player-2,1|stat:def|by:1",
            "unboost|mon:Pikachu,player-2,2|stat:def|by:1",
            "move|mon:Pikachu,player-2,1|name:Tail Whip|spread:Pikachu,player-1,1;Pikachu,player-1,2",
            "unboost|mon:Pikachu,player-1,1|stat:def|by:1",
            "unboost|mon:Pikachu,player-1,2|stat:def|by:1",
            "residual",
            "turn|turn:2"
        ]"#).unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn modified_speed_impacts_order() {
    let team_1: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Agility",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap();
    let team_2: TeamData = serde_json::from_str(
        r#"{
        "members": [
            {
                "name": "Pikachu",
                "species": "Pikachu",
                "ability": "No Ability",
                "moves": [
                    "Agility",
                    "Tackle"
                ],
                "nature": "Timid",
                "gender": "M",
                "level": 50
            }
        ]
    }"#,
    )
    .unwrap();
    let mut battle = make_singles_battle(team_1, team_2, 0).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[

            "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:73/95",
            "damage|mon:Pikachu,player-1,1|health:77/100",
            "move|mon:Pikachu,player-1,1|name:Agility|target:Pikachu,player-1,1",
            "boost|mon:Pikachu,player-1,1|stat:spe|by:2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:73/95",
            "damage|mon:Pikachu,player-2,1|health:77/100",
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:54/95",
            "damage|mon:Pikachu,player-1,1|health:57/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:51/95",
            "damage|mon:Pikachu,player-2,1|health:54/100",
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:34/95",
            "damage|mon:Pikachu,player-1,1|health:36/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:27/95",
            "damage|mon:Pikachu,player-2,1|health:29/100",
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Pikachu,player-1,1",
            "split|side:0",
            "damage|mon:Pikachu,player-1,1|health:12/95",
            "damage|mon:Pikachu,player-1,1|health:13/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
