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

fn probopass() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Probopass",
                    "species": "Probopass",
                    "ability": "Levitate",
                    "moves": [
                        "Gravity",
                        "Earthquake",
                        "Magnet Rise"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn staraptor() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Staraptor",
                    "species": "Staraptor",
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

fn probopass_hawlucha() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Probopass",
                    "species": "Probopass",
                    "ability": "Levitate",
                    "moves": [
                        "Gravity"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },{
                    "name": "Hawlucha",
                    "species": "Hawlucha",
                    "ability": "No Ability",
                    "moves": [
                        "Sky Drop"
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
fn gravity_grounds_flying_types() {
    let mut battle = make_battle(
        BattleType::Singles,
        0,
        probopass().unwrap(),
        staraptor().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Probopass,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Earthquake",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|health:110/145",
            "damage|mon:Staraptor,player-2,1|health:76/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gravity_negates_levitate() {
    let mut battle = make_battle(
        BattleType::Singles,
        0,
        probopass().unwrap(),
        probopass().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Probopass,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Earthquake",
            "supereffective|mon:Probopass,player-2,1",
            "split|side:1",
            "damage|mon:Probopass,player-2,1|health:48/120",
            "damage|mon:Probopass,player-2,1|health:40/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gravity_removes_magnet_rise() {
    let mut team = probopass().unwrap();
    team.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(BattleType::Singles, 0, team, probopass().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Probopass,player-1,1|name:Magnet Rise|target:Probopass,player-1,1",
            "start|mon:Probopass,player-1,1|move:Magnet Rise",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "end|mon:Probopass,player-1,1|move:Magnet Rise",
            "move|mon:Probopass,player-2,1|name:Earthquake",
            "supereffective|mon:Probopass,player-1,1",
            "split|side:0",
            "damage|mon:Probopass,player-1,1|health:48/120",
            "damage|mon:Probopass,player-1,1|health:40/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gravity_cancels_and_disables_fly_after_used() {
    let mut battle = make_battle(
        BattleType::Singles,
        0,
        probopass().unwrap(),
        staraptor().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Staraptor,player-2,1|name:Fly|noanim",
            "prepare|mon:Staraptor,player-2,1|move:Fly",
            "move|mon:Probopass,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "activate|mon:Staraptor,player-2,1|move:Gravity",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Staraptor,player-2,1|name:Struggle|target:Probopass,player-1,1",
            "crit|mon:Probopass,player-1,1",
            "split|side:0",
            "damage|mon:Probopass,player-1,1|health:94/120",
            "damage|mon:Probopass,player-1,1|health:79/100",
            "split|side:1",
            "damage|mon:Staraptor,player-2,1|from:Struggle Recoil|health:109/145",
            "damage|mon:Staraptor,player-2,1|from:Struggle Recoil|health:76/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gravity_cancels_fly_before_first_use() {
    let mut staraptor = staraptor().unwrap();
    staraptor.members[0].level = 1;
    let mut battle = make_battle(BattleType::Singles, 0, probopass().unwrap(), staraptor).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Probopass,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "cant|mon:Staraptor,player-2,1|from:move:Gravity",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gravity_cancels_fly_before_second_use() {
    let mut staraptor = staraptor().unwrap();
    staraptor.members[0].level = 1;
    let mut battle = make_battle(BattleType::Singles, 0, probopass().unwrap(), staraptor).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Staraptor,player-2,1|name:Fly|noanim",
            "prepare|mon:Staraptor,player-2,1|move:Fly",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "activate|mon:Staraptor,player-2,1|move:Gravity",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gravity_cancels_sky_drop() {
    let mut battle = make_battle(
        BattleType::Doubles,
        0,
        probopass_hawlucha().unwrap(),
        probopass_hawlucha().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hawlucha,player-2,2|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-2,2|move:Sky Drop|target:Hawlucha,player-1,2",
            "move|mon:Probopass,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "activate|mon:Hawlucha,player-2,2|move:Gravity",
            "end|mon:Hawlucha,player-1,2|move:Sky Drop",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gravity_cancels_sky_drop_before_second_use() {
    let mut team = probopass_hawlucha().unwrap();
    team.members[1].level = 1;
    let mut battle =
        make_battle(BattleType::Doubles, 0, probopass_hawlucha().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hawlucha,player-2,2|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-2,2|move:Sky Drop|target:Hawlucha,player-1,2",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Probopass,player-1,1|name:Gravity",
            "fieldstart|move:Gravity",
            "activate|mon:Hawlucha,player-2,2|move:Gravity",
            "end|mon:Hawlucha,player-1,2|move:Sky Drop",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
