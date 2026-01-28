use anyhow::Result;
use battler::{
    BattleType,
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
                    "name": "Probopass",
                    "species": "Probopass",
                    "ability": "Sturdy",
                    "moves": [
                        "Smack Down",
                        "Earthquake",
                        "Magnet Rise",
                        "Telekinesis"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn braviary() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Braviary",
                    "species": "Braviary",
                    "ability": "No Ability",
                    "moves": [
                        "Fly"
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
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
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
fn smack_down_grounds_flying_types() {
    let mut battle =
        make_battle(BattleType::Singles, 0, team().unwrap(), braviary().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Probopass,player-1,1|name:Smack Down|target:Braviary,player-2,1",
            "supereffective|mon:Braviary,player-2,1",
            "split|side:1",
            "damage|mon:Braviary,player-2,1|health:110/160",
            "damage|mon:Braviary,player-2,1|health:69/100",
            "activate|mon:Braviary,player-2,1|move:Smack Down",
            "start|mon:Braviary,player-2,1|move:Smack Down",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Earthquake",
            "split|side:1",
            "damage|mon:Braviary,player-2,1|health:79/160",
            "damage|mon:Braviary,player-2,1|health:50/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn smack_down_negates_levitate() {
    let mut team_p2 = team().unwrap();
    team_p2.members[0].ability = "Levitate".to_owned();

    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team_p2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Probopass,player-1,1|name:Smack Down|target:Probopass,player-2,1",
            "resisted|mon:Probopass,player-2,1",
            "split|side:1",
            "damage|mon:Probopass,player-2,1|health:114/120",
            "damage|mon:Probopass,player-2,1|health:95/100",
            "activate|mon:Probopass,player-2,1|move:Smack Down",
            "start|mon:Probopass,player-2,1|move:Smack Down",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Earthquake",
            "supereffective|mon:Probopass,player-2,1",
            "split|side:1",
            "damage|mon:Probopass,player-2,1|health:46/120",
            "damage|mon:Probopass,player-2,1|health:39/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn smack_down_removes_magnet_rise() {
    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Probopass,player-2,1|name:Magnet Rise|target:Probopass,player-2,1",
            "start|mon:Probopass,player-2,1|move:Magnet Rise",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Smack Down|target:Probopass,player-2,1",
            "resisted|mon:Probopass,player-2,1",
            "split|side:1",
            "damage|mon:Probopass,player-2,1|health:114/120",
            "damage|mon:Probopass,player-2,1|health:95/100",
            "activate|mon:Probopass,player-2,1|move:Smack Down",
            "end|mon:Probopass,player-2,1|move:Magnet Rise",
            "start|mon:Probopass,player-2,1|move:Smack Down",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Probopass,player-1,1|name:Earthquake",
            "supereffective|mon:Probopass,player-2,1",
            "split|side:1",
            "damage|mon:Probopass,player-2,1|health:46/120",
            "damage|mon:Probopass,player-2,1|health:39/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn smack_down_hits_and_cancels_fly_lagging_tail() {
    let mut team_p2 = braviary().unwrap();
    team_p2.members[0].item = Some("Lagging Tail".to_owned());

    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team_p2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Braviary,player-2,1|name:Fly|noanim",
            "prepare|mon:Braviary,player-2,1|move:Fly",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Smack Down|target:Braviary,player-2,1",
            "supereffective|mon:Braviary,player-2,1",
            "split|side:1",
            "damage|mon:Braviary,player-2,1|health:110/160",
            "damage|mon:Braviary,player-2,1|health:69/100",
            "activate|mon:Braviary,player-2,1|move:Smack Down",
            "start|mon:Braviary,player-2,1|move:Smack Down",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn smack_down_removes_telekinesis() {
    let mut battle = make_battle(BattleType::Singles, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Probopass,player-1,1|name:Telekinesis|target:Probopass,player-2,1",
            "start|mon:Probopass,player-2,1|move:Telekinesis",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Earthquake|noanim",
            "immune|mon:Probopass,player-2,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Probopass,player-1,1|name:Smack Down|target:Probopass,player-2,1",
            "resisted|mon:Probopass,player-2,1",
            "crit|mon:Probopass,player-2,1",
            "split|side:1",
            "damage|mon:Probopass,player-2,1|health:111/120",
            "damage|mon:Probopass,player-2,1|health:93/100",
            "activate|mon:Probopass,player-2,1|move:Smack Down",
            "end|mon:Probopass,player-2,1|move:Telekinesis",
            "start|mon:Probopass,player-2,1|move:Smack Down",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Probopass,player-1,1|name:Earthquake",
            "supereffective|mon:Probopass,player-2,1",
            "split|side:1",
            "damage|mon:Probopass,player-2,1|health:35/120",
            "damage|mon:Probopass,player-2,1|health:30/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
