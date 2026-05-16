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
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Guzzlord",
                    "species": "Guzzlord",
                    "ability": "No Ability",
                    "moves": [
                        "Stomping Tantrum",
                        "Recover",
                        "Rest",
                        "Attract",
                        "Spore",
                        "Protect",
                        "Earthquake",
                        "Hyper Beam",
                        "Fly",
                        "Smack Down",
                        "Dive",
                        "Electric Terrain",
                        "Roost",
                        "Shore Up",
                        "Gravity",
                        "Splash",
                        "Celebrate"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Celesteela",
                    "species": "Celesteela",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn stomping_tantrum_doubles_power_if_last_move_failed() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "move|mon:Guzzlord,player-2,1|name:Recover|target:Guzzlord,player-2,1",
            "split|side:1",
            "heal|mon:Guzzlord,player-2,1|health:556/556",
            "heal|mon:Guzzlord,player-2,1|health:100/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Attract|noanim",
            "fail|mon:Guzzlord,player-1,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:320/556",
            "damage|mon:Guzzlord,player-2,1|health:58/100",
            "move|mon:Guzzlord,player-2,1|name:Recover|target:Guzzlord,player-2,1",
            "split|side:1",
            "heal|mon:Guzzlord,player-2,1|health:556/556",
            "heal|mon:Guzzlord,player-2,1|health:100/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "move|mon:Guzzlord,player-2,1|name:Rest|target:Guzzlord,player-2,1",
            "status|mon:Guzzlord,player-2,1|status:Sleep",
            "split|side:1",
            "heal|mon:Guzzlord,player-2,1|health:556/556",
            "heal|mon:Guzzlord,player-2,1|health:100/100",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Spore|noanim",
            "fail|mon:Guzzlord,player-1,1",
            "residual",
            "turn|turn:6",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:320/556",
            "damage|mon:Guzzlord,player-2,1|health:58/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_does_not_double_due_to_hitting_protect() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-2,1|name:Protect|target:Guzzlord,player-2,1",
            "singleturn|mon:Guzzlord,player-2,1|move:Protect",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|noanim",
            "activate|mon:Guzzlord,player-2,1|move:Protect",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_doubles_power_if_one_target_immune_other_protects() {
    let mut battle = make_battle(0, BattleType::Doubles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 6;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-2,1|name:Protect|target:Guzzlord,player-2,1",
            "singleturn|mon:Guzzlord,player-2,1|move:Protect",
            "move|mon:Guzzlord,player-1,1|name:Earthquake|noanim",
            "activate|mon:Guzzlord,player-2,1|move:Protect",
            "immune|mon:Celesteela,player-1,2",
            "immune|mon:Celesteela,player-2,2",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:320/556",
            "damage|mon:Guzzlord,player-2,1|health:58/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_does_not_double_if_recharging() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 7"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Hyper Beam|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:329/556",
            "damage|mon:Guzzlord,player-2,1|health:60/100",
            "activate|mon:Guzzlord,player-1,1|condition:Must Recharge",
            "move|mon:Guzzlord,player-2,1|name:Recover|target:Guzzlord,player-2,1",
            "split|side:1",
            "heal|mon:Guzzlord,player-2,1|health:556/556",
            "heal|mon:Guzzlord,player-2,1|health:100/100",
            "residual",
            "turn|turn:2",
            "continue",
            "cant|mon:Guzzlord,player-1,1|from:Must Recharge",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_does_not_double_if_fly_interrupted_by_smack_down() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 8"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 9"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Fly|noanim",
            "prepare|mon:Guzzlord,player-1,1|move:Fly",
            "move|mon:Guzzlord,player-2,1|name:Smack Down|target:Guzzlord,player-1,1",
            "split|side:0",
            "damage|mon:Guzzlord,player-1,1|health:476/556",
            "damage|mon:Guzzlord,player-1,1|health:86/100",
            "activate|mon:Guzzlord,player-1,1|move:Smack Down",
            "start|mon:Guzzlord,player-1,1|move:Smack Down",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_doubles_if_two_turn_move_fails() {
    let mut team = team().unwrap();
    team.members[0].ability = "Water Absorb".to_owned();
    let mut battle = make_battle(0, BattleType::Singles, team.clone(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 10"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Dive|noanim",
            "prepare|mon:Guzzlord,player-1,1|move:Dive",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Dive|noanim",
            "immune|mon:Guzzlord,player-2,1|from:ability:Water Absorb",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:320/556",
            "damage|mon:Guzzlord,player-2,1|health:58/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_doubles_power_if_rest_fails_directly_from_move_logic() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Comatose".to_owned();
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Rest|noanim",
            "fail|mon:Guzzlord,player-1,1|what:heal",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:320/556",
            "damage|mon:Guzzlord,player-2,1|health:58/100",
            "move|mon:Guzzlord,player-2,1|name:Rest|noanim",
            "fail|mon:Guzzlord,player-2,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Guzzlord,player-2,1|name:Stomping Tantrum|target:Guzzlord,player-1,1",
            "split|side:0",
            "damage|mon:Guzzlord,player-1,1|health:320/556",
            "damage|mon:Guzzlord,player-1,1|health:58/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_does_not_double_if_rest_fails_from_terrain() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 11"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Electric Terrain",
            "fieldstart|move:Electric Terrain",
            "move|mon:Guzzlord,player-2,1|name:Stomping Tantrum|target:Guzzlord,player-1,1",
            "split|side:0",
            "damage|mon:Guzzlord,player-1,1|health:437/556",
            "damage|mon:Guzzlord,player-1,1|health:79/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Rest|noanim",
            "activate|mon:Guzzlord,player-1,1|move:Electric Terrain",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_does_not_double_when_healing_fails() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 12"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 13"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Roost|noanim",
            "fail|mon:Guzzlord,player-1,1|what:heal",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "move|mon:Guzzlord,player-2,1|name:Recover|target:Guzzlord,player-2,1",
            "split|side:1",
            "heal|mon:Guzzlord,player-2,1|health:556/556",
            "heal|mon:Guzzlord,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Shore Up|noanim",
            "fail|mon:Guzzlord,player-1,1|what:heal",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_doubles_power_if_move_fails_due_to_gravity() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 14"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 15"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "cant|mon:Guzzlord,player-2,1|from:move:Gravity",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-2,1|name:Stomping Tantrum|target:Guzzlord,player-1,1",
            "split|side:0",
            "damage|mon:Guzzlord,player-1,1|health:320/556",
            "damage|mon:Guzzlord,player-1,1|health:58/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn stomping_tantrum_does_not_double_from_splash_or_celebrate() {
    let mut battle = make_battle(0, BattleType::Singles, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 15"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 16"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Guzzlord,player-1,1|name:Splash|target:Guzzlord,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "move|mon:Guzzlord,player-2,1|name:Recover|target:Guzzlord,player-2,1",
            "split|side:1",
            "heal|mon:Guzzlord,player-2,1|health:556/556",
            "heal|mon:Guzzlord,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Celebrate|target:Guzzlord,player-1,1",
            "activate|mon:Guzzlord,player-1,1|move:Celebrate",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Guzzlord,player-1,1|name:Stomping Tantrum|target:Guzzlord,player-2,1",
            "split|side:1",
            "damage|mon:Guzzlord,player-2,1|health:437/556",
            "damage|mon:Guzzlord,player-2,1|health:79/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
