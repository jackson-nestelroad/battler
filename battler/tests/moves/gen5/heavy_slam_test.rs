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
    static_local_data_store,
};

fn aggron() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Aggron",
                    "species": "Aggron",
                    "ability": "No Ability",
                    "moves": [
                        "Heavy Slam"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn target_team(species: &str, moves: &str) -> TeamData {
    let mut team: TeamData = serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Target",
                    "species": "Aggron",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 100
                }
            ]
        }"#,
    )
    .unwrap();
    team.members[0].name = species.to_owned();
    team.members[0].species = species.to_owned();
    if !moves.is_empty() {
        team.members[0].moves = vec![moves.to_owned()];
    }
    team
}

fn make_battle(
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>, anyhow::Error> {
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
        .build(static_local_data_store())
}

#[test]
fn heavy_slam_vs_minccino() {
    // 360.0 / 5.8 = 62.06 (120 BP)
    let mut battle = make_battle(0, aggron(), target_team("Minccino", "")).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Aggron,player-1,1|name:Heavy Slam|target:Minccino,player-2,1",
            "split|side:1",
            "damage|mon:Minccino,player-2,1|health:115/220",
            "damage|mon:Minccino,player-2,1|health:53/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heavy_slam_vs_cofagrigus() {
    // 360.0 / 76.5 = 4.70 (100 BP)
    let mut battle = make_battle(0, aggron(), target_team("Cofagrigus", "")).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Aggron,player-1,1|name:Heavy Slam|target:Cofagrigus,player-2,1",
            "split|side:1",
            "damage|mon:Cofagrigus,player-2,1|health:199/226",
            "damage|mon:Cofagrigus,player-2,1|health:89/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heavy_slam_vs_krookodile() {
    // 360.0 / 96.3 = 3.73 (80 BP)
    let mut battle = make_battle(0, aggron(), target_team("Krookodile", "")).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Aggron,player-1,1|name:Heavy Slam|target:Krookodile,player-2,1",
            "split|side:1",
            "damage|mon:Krookodile,player-2,1|health:263/300",
            "damage|mon:Krookodile,player-2,1|health:88/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heavy_slam_vs_druddigon() {
    // 360.0 / 139.0 = 2.58 (60 BP)
    let mut battle = make_battle(0, aggron(), target_team("Druddigon", "")).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Aggron,player-1,1|name:Heavy Slam|target:Druddigon,player-2,1",
            "split|side:1",
            "damage|mon:Druddigon,player-2,1|health:239/264",
            "damage|mon:Druddigon,player-2,1|health:91/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heavy_slam_vs_golurk() {
    // 360.0 / 330.0 = 1.09 (40 BP)
    let mut battle = make_battle(0, aggron(), target_team("Golurk", "")).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Aggron,player-1,1|name:Heavy Slam|target:Golurk,player-2,1",
            "split|side:1",
            "damage|mon:Golurk,player-2,1|health:269/288",
            "damage|mon:Golurk,player-2,1|health:94/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn heavy_slam_doubles_damage_against_minimize() {
    let mut battle = make_battle(
        222222222222222,
        aggron(),
        target_team("Minccino", "Minimize"),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // 120 BP * 2 = 240 BP
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Aggron,player-1,1|name:Heavy Slam|target:Minccino,player-2,1",
            "split|side:1",
            "damage|mon:Minccino,player-2,1|health:20/220",
            "damage|mon:Minccino,player-2,1|health:10/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
