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
    get_controlled_rng_for_battle,
    static_local_data_store,
};

fn golisopod_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Golisopod",
                    "species": "Golisopod",
                    "ability": "Emergency Exit",
                    "moves": [
                        "Tackle",
                        "Take Down",
                        "Struggle",
                        "Belly Drum",
                        "Double Kick"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Decidueye",
                    "species": "Decidueye",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Incineroar",
                    "species": "Incineroar",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn pikachu_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Thunderbolt",
                        "Tackle",
                        "Discharge",
                        "Confuse Ray",
                        "Toxic",
                        "Fury Attack",
                        "U-turn",
                        "Sky Drop",
                        "Dragon Tail",
                        "Spikes",
                        "Stealth Rock",
                        "Sandstorm",
                        "Bind",
                        "Leech Seed",
                        "Quash",
                        "Pursuit"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Pursuit",
                        "Bestow"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Meloetta",
                    "species": "Meloetta",
                    "ability": "No Ability",
                    "moves": [
                        "Relic Song"
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
    seed: u64,
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn emergency_exit_does_not_activate_if_player_cannot_switch() {
    let mut team = golisopod_team().unwrap();
    team.members.drain(1..);
    let mut battle = make_battle(0, BattleType::Singles, team, pikachu_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:63/135",
            "damage|mon:Golisopod,player-1,1|health:47/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_on_faint() {
    let mut team = golisopod_team().unwrap();
    team.members[0].level = 1;
    let mut battle = make_battle(0, BattleType::Singles, team, pikachu_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:0",
            "damage|mon:Golisopod,player-1,1|health:0",
            "faint|mon:Golisopod,player-1,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_when_hp_goes_below_half() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:63/135",
            "damage|mon:Golisopod,player-1,1|health:47/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Decidueye"],
            ["switch", "player-1", "Decidueye"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_if_berry_heals() {
    let mut team = golisopod_team().unwrap();
    team.members[0].item = Some("Sitrus Berry".to_owned());
    let mut battle = make_battle(0, BattleType::Singles, team, pikachu_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:63/135",
            "damage|mon:Golisopod,player-1,1|health:47/100",
            "itemend|mon:Golisopod,player-1,1|item:Sitrus Berry|eat",
            "split|side:0",
            "heal|mon:Golisopod,player-1,1|from:item:Sitrus Berry|health:96/135",
            "heal|mon:Golisopod,player-1,1|from:item:Sitrus Berry|health:72/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:24/135",
            "damage|mon:Golisopod,player-1,1|health:18/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_if_hp_already_below_half() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:63/135",
            "damage|mon:Golisopod,player-1,1|health:47/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Decidueye"],
            ["switch", "player-1", "Decidueye"],
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Golisopod"],
            ["switch", "player-1", "Golisopod"],
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:54/135",
            "damage|mon:Golisopod,player-1,1|health:40/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_on_confusion_damage() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-2,1|name:Confuse Ray|target:Golisopod,player-1,1",
            "start|mon:Golisopod,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:3",
            "continue",
            "activate|mon:Golisopod,player-1,1|condition:Confusion",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:Confusion|health:52/135",
            "damage|mon:Golisopod,player-1,1|from:Confusion|health:39/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_on_toxic_damage() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-2,1|name:Toxic|target:Golisopod,player-1,1",
            "status|mon:Golisopod,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:61/135",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:46/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_if_battle_is_ending() {
    let mut team = pikachu_team().unwrap();
    team.members.drain(1..);
    let mut battle = make_battle(0, BattleType::Singles, golisopod_team().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Golisopod,player-1,1|name:Take Down|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:0",
            "damage|mon:Pikachu,player-2,1|health:0",
            "faint|mon:Pikachu,player-2,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:Recoil|health:45/135",
            "damage|mon:Golisopod,player-1,1|from:Recoil|health:34/100",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_on_recoil() {
    let mut team = pikachu_team().unwrap();
    team.members[0].ability = "Sturdy".to_owned();
    let mut battle = make_battle(0, BattleType::Singles, golisopod_team().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Golisopod,player-1,1|name:Take Down|target:Pikachu,player-2,1",
            "activate|mon:Pikachu,player-2,1|ability:Sturdy",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:1/95",
            "damage|mon:Pikachu,player-2,1|health:2/100",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:Recoil|health:45/135",
            "damage|mon:Golisopod,player-1,1|from:Recoil|health:34/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_on_struggle_recoil() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Golisopod,player-1,1|name:Struggle|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:30/95",
            "damage|mon:Pikachu,player-2,1|health:32/100",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:Struggle Recoil|health:35/135",
            "damage|mon:Golisopod,player-1,1|from:Struggle Recoil|health:26/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_on_belly_drum() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Golisopod,player-1,1|name:Belly Drum|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:2/135",
            "damage|mon:Golisopod,player-1,1|health:2/100",
            "boost|mon:Golisopod,player-1,1|stat:atk|by:6|max",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_when_hit_by_sheer_force() {
    let mut team = pikachu_team().unwrap();
    team.members[0].ability = "Sheer Force".to_owned();
    let mut battle = make_battle(0, BattleType::Singles, golisopod_team().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:43/135",
            "damage|mon:Golisopod,player-1,1|health:32/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_after_last_hit_of_multihit_move() {
    let mut team = pikachu_team().unwrap();
    team.members[0].ability = "Skill Link".to_owned();
    let mut battle = make_battle(0, BattleType::Singles, golisopod_team().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-2,1|name:Fury Attack|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:65/135",
            "damage|mon:Golisopod,player-1,1|health:49/100",
            "animatemove|mon:Pikachu,player-2,1|name:Fury Attack|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:61/135",
            "damage|mon:Golisopod,player-1,1|health:46/100",
            "animatemove|mon:Pikachu,player-2,1|name:Fury Attack|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:57/135",
            "damage|mon:Golisopod,player-1,1|health:43/100",
            "animatemove|mon:Pikachu,player-2,1|name:Fury Attack|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:53/135",
            "damage|mon:Golisopod,player-1,1|health:40/100",
            "animatemove|mon:Pikachu,player-2,1|name:Fury Attack|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:49/135",
            "damage|mon:Golisopod,player-1,1|health:37/100",
            "hitcount|hits:5",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_blocks_user_switch() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 6"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-2,1|name:U-turn|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:55/135",
            "damage|mon:Golisopod,player-1,1|health:41/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Decidueye"],
            ["switch", "player-1", "Decidueye"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_during_sky_drop() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 7"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Toxic|target:Golisopod,player-1,1",
            "status|mon:Golisopod,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:127/135",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:95/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:111/135",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:83/100",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:86/135",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:64/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Pikachu,player-2,1|name:Sky Drop|noanim",
            "prepare|mon:Pikachu,player-2,1|move:Sky Drop|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:53/135",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:40/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_pursuit() {
    let mut battle = make_battle(
        0,
        BattleType::Doubles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:63/135",
            "damage|mon:Golisopod,player-1,1|health:47/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "activate|mon:Golisopod,player-1,1|move:Pursuit",
            "move|mon:Pikachu,player-2,2|name:Pursuit|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:47/135",
            "damage|mon:Golisopod,player-1,1|health:35/100",
            "switchout|mon:Golisopod,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Incineroar"],
            ["switch", "player-1", "Incineroar"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_does_not_activate_before_force_switch() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 8"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Pikachu,player-2,1|name:Dragon Tail|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:57/135",
            "damage|mon:Golisopod,player-1,1|health:43/100",
            "split|side:0",
            ["drag", "player-1", "Decidueye"],
            ["drag", "player-1", "Decidueye"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_between_hazards() {
    let mut battle = make_battle(
        0,
        BattleType::Singles,
        golisopod_team().unwrap(),
        pikachu_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 9"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 10"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Discharge",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:69/135",
            "damage|mon:Golisopod,player-1,1|health:52/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Decidueye"],
            ["switch", "player-1", "Decidueye"],
            "move|mon:Pikachu,player-2,1|name:Spikes",
            "sidestart|side:0|move:Spikes|count:1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Pikachu,player-2,1|name:Stealth Rock",
            "sidestart|side:0|move:Stealth Rock",
            "residual",
            "turn|turn:4",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Golisopod"],
            ["switch", "player-1", "Golisopod"],
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:move:Spikes|health:53/135",
            "damage|mon:Golisopod,player-1,1|from:move:Spikes|health:40/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Decidueye"],
            ["switch", "player-1", "Decidueye"],
            "split|side:0",
            "damage|mon:Decidueye,player-1,1|from:move:Spikes|health:121/138",
            "damage|mon:Decidueye,player-1,1|from:move:Spikes|health:88/100",
            "split|side:0",
            "damage|mon:Decidueye,player-1,1|from:move:Stealth Rock|health:104/138",
            "damage|mon:Decidueye,player-1,1|from:move:Stealth Rock|health:76/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_between_residuals() {
    let mut team = pikachu_team().unwrap();
    team.members[1].item = Some("Sticky Barb".to_owned());
    let mut battle = make_battle(0, BattleType::Doubles, golisopod_team().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 11;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 4,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 12,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 13,2;move 1,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Leech Seed|target:Incineroar,player-1,2",
            "start|mon:Incineroar,player-1,2|move:Leech Seed",
            "move|mon:Pikachu,player-2,2|name:Bestow|target:Golisopod,player-1,1",
            "itemend|mon:Pikachu,player-2,2|item:Sticky Barb|from:move:Bestow",
            "item|mon:Golisopod,player-1,1|item:Sticky Barb|from:move:Bestow|of:Pikachu,player-2,2",
            "weather|weather:Sandstorm|residual",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|from:weather:Sandstorm|health:75/95",
            "damage|mon:Pikachu,player-2,1|from:weather:Sandstorm|health:79/100",
            "split|side:1",
            "damage|mon:Pikachu,player-2,2|from:weather:Sandstorm|health:42/95",
            "damage|mon:Pikachu,player-2,2|from:weather:Sandstorm|health:45/100",
            "split|side:0",
            "damage|mon:Incineroar,player-1,2|from:weather:Sandstorm|health:94/155",
            "damage|mon:Incineroar,player-1,2|from:weather:Sandstorm|health:61/100",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:weather:Sandstorm|health:79/135",
            "damage|mon:Golisopod,player-1,1|from:weather:Sandstorm|health:59/100",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:54/135",
            "damage|mon:Golisopod,player-1,1|from:status:Bad Poison|health:40/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Incineroar,player-1,2|from:move:Bind|health:75/155",
            "damage|mon:Incineroar,player-1,2|from:move:Bind|health:49/100",
            "split|side:0",
            "damage|mon:Incineroar,player-1,2|from:move:Leech Seed|health:56/155",
            "damage|mon:Incineroar,player-1,2|from:move:Leech Seed|health:37/100",
            "split|side:1",
            "heal|mon:Pikachu,player-2,1|from:move:Leech Seed|of:Incineroar,player-1,2|health:94/95",
            "heal|mon:Pikachu,player-2,1|from:move:Leech Seed|of:Incineroar,player-1,2|health:99/100",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 4, &expected_logs);
}

#[test]
fn emergency_exit_activates_during_move() {
    let mut golisopod_team = golisopod_team().unwrap();
    golisopod_team.members[0].item = Some("Life Orb".to_owned());
    let mut pikachu_team = pikachu_team().unwrap();
    pikachu_team.members[0].ability = "Rough Skin".to_owned();
    pikachu_team.members[0].level = 60;
    let mut battle = make_battle(0, BattleType::Doubles, golisopod_team, pikachu_team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 4,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Golisopod,player-1,1|name:Double Kick|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:68/112",
            "damage|mon:Pikachu,player-2,1|health:61/100",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:ability:Rough Skin|of:Pikachu,player-2,1|health:53/135",
            "damage|mon:Golisopod,player-1,1|from:ability:Rough Skin|of:Pikachu,player-2,1|health:40/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "switchout|mon:Golisopod,player-1,1",
            "hitcount|hits:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn emergency_exit_activates_pursuit_during_move() {
    let mut golisopod_team = golisopod_team().unwrap();
    golisopod_team.members[0].item = Some("Life Orb".to_owned());
    let mut pikachu_team = pikachu_team().unwrap();
    pikachu_team.members[0].ability = "Rough Skin".to_owned();
    pikachu_team.members[0].level = 60;
    let mut battle = make_battle(0, BattleType::Doubles, golisopod_team, pikachu_team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 4,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 14,-2;move 0,2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Quash|target:Pikachu,player-2,2",
            "activate|mon:Pikachu,player-2,2|move:Quash",
            "move|mon:Golisopod,player-1,1|name:Double Kick|target:Pikachu,player-2,1",
            "split|side:1",
            "damage|mon:Pikachu,player-2,1|health:68/112",
            "damage|mon:Pikachu,player-2,1|health:61/100",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|from:ability:Rough Skin|of:Pikachu,player-2,1|health:53/135",
            "damage|mon:Golisopod,player-1,1|from:ability:Rough Skin|of:Pikachu,player-2,1|health:40/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "activate|mon:Golisopod,player-1,1|move:Pursuit",
            "move|mon:Pikachu,player-2,2|name:Pursuit|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:37/135",
            "damage|mon:Golisopod,player-1,1|health:28/100",
            "switchout|mon:Golisopod,player-1,1",
            "hitcount|hits:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn emergency_exit_activates_pursuit_during_opponent_move() {
    let mut pikachu_team = pikachu_team().unwrap();
    pikachu_team.members[0].level = 5;
    pikachu_team.members[2].item = Some("Life Orb".to_owned());
    pikachu_team.members.swap(1, 2);
    let mut battle = make_battle(
        0,
        BattleType::Doubles,
        golisopod_team().unwrap(),
        pikachu_team,
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 15,2;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Meloetta,player-2,2|name:Relic Song|spread:Golisopod,player-1,1",
            "immune|mon:Decidueye,player-1,2",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:65/135",
            "damage|mon:Golisopod,player-1,1|health:49/100",
            "activate|mon:Golisopod,player-1,1|ability:Emergency Exit",
            "activate|mon:Golisopod,player-1,1|move:Pursuit",
            "move|mon:Pikachu,player-2,1|name:Pursuit|target:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:63/135",
            "damage|mon:Golisopod,player-1,1|health:47/100",
            "switchout|mon:Golisopod,player-1,1",
            "formechange|mon:Meloetta,player-2,2|species:Meloetta-Pirouette|from:move:Relic Song",
            "split|side:1",
            "damage|mon:Meloetta,player-2,2|from:item:Life Orb|health:144/160",
            "damage|mon:Meloetta,player-2,2|from:item:Life Orb|health:90/100",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Incineroar"],
            ["switch", "player-1", "Incineroar"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn emergency_exit_activates_after_eject_button() {
    let mut team = golisopod_team().unwrap();
    team.members[0].item = Some("Eject Button".to_owned());
    let mut battle = make_battle(0, BattleType::Singles, team, pikachu_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Golisopod,player-1,1",
            "supereffective|mon:Golisopod,player-1,1",
            "split|side:0",
            "damage|mon:Golisopod,player-1,1|health:63/135",
            "damage|mon:Golisopod,player-1,1|health:47/100",
            "itemend|mon:Golisopod,player-1,1|item:Eject Button",
            "switchout|mon:Golisopod,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Decidueye"],
            ["switch", "player-1", "Decidueye"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
