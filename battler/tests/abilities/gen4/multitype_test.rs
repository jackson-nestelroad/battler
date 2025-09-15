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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn arceus() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Arceus",
                    "species": "Arceus",
                    "ability": "Multitype",
                    "moves": [
                        "Judgment"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn kecleon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Kecleon",
                    "species": "Kecleon",
                    "ability": "Color Change",
                    "moves": [
                        "Trick"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn ditto() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "No Ability",
                    "moves": [
                        "Transform"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
fn multitype_changes_arceus_forme_dynamically_if_incorrect() {
    let mut team_1 = arceus().unwrap();
    team_1.members[0].item = Some("Flame Plate".to_owned());
    let mut team_2 = arceus().unwrap();
    team_2.members[0].item = Some("Splash Plate".to_owned());
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Arceus"],
            ["switch", "player-1", "Arceus"],
            "split|side:1",
            ["switch", "player-2", "Arceus"],
            ["switch", "player-2", "Arceus"],
            "split|side:0",
            ["specieschange", "player-1", "species:Arceus-Fire"],
            ["specieschange", "player-1", "species:Arceus-Fire"],
            "formechange|mon:Arceus,player-1,1|species:Arceus-Fire|from:ability:Multitype",
            "split|side:1",
            ["specieschange", "player-2", "species:Arceus-Water"],
            ["specieschange", "player-2", "species:Arceus-Water"],
            "formechange|mon:Arceus,player-2,1|species:Arceus-Water|from:ability:Multitype",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn multitype_does_not_change_arceus_forme_if_correct() {
    let mut team_1 = arceus().unwrap();
    team_1.members[0].species = "Arceus-Fire".to_owned();
    team_1.members[0].item = Some("Flame Plate".to_owned());
    let mut team_2 = arceus().unwrap();
    team_2.members[0].species = "Arceus-Water".to_owned();
    team_2.members[0].item = Some("Splash Plate".to_owned());
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Arceus-Fire"],
            ["switch", "player-1", "Arceus-Fire"],
            "split|side:1",
            ["switch", "player-2", "Arceus-Water"],
            ["switch", "player-2", "Arceus-Water"],
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn multitype_works_for_non_arceus() {
    let mut team_1 = arceus().unwrap();
    team_1.members[0].species = "Arceus-Water".to_owned();
    team_1.members[0].item = Some("Splash Plate".to_owned());
    let mut team_2 = kecleon().unwrap();
    team_2.members[0].ability = "Multitype".to_owned();
    team_2.members[0].item = Some("Splash Plate".to_owned());
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Arceus,player-1,1|name:Judgment|target:Kecleon,player-2,1",
            "resisted|mon:Kecleon,player-2,1",
            "split|side:1",
            "damage|mon:Kecleon,player-2,1|health:81/120",
            "damage|mon:Kecleon,player-2,1|health:68/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn multitype_does_not_change_types_when_transformed() {
    let mut team_1 = arceus().unwrap();
    team_1.members[0].species = "Arceus-Water".to_owned();
    team_1.members[0].item = Some("Splash Plate".to_owned());
    let mut battle = make_battle(0, team_1, ditto().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-2,1|name:Transform|target:Arceus,player-1,1",
            "transform|mon:Ditto,player-2,1|into:Arceus,player-1,1|species:Arceus-Water",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Arceus,player-1,1|name:Judgment|target:Ditto,player-2,1",
            "resisted|mon:Ditto,player-2,1",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:69/108",
            "damage|mon:Ditto,player-2,1|health:64/100",
            "move|mon:Ditto,player-2,1|name:Judgment|target:Arceus,player-1,1",
            "split|side:0",
            "damage|mon:Arceus,player-1,1|health:139/180",
            "damage|mon:Arceus,player-1,1|health:78/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn multitype_does_not_allow_plate_to_be_taken() {
    let mut team_1 = arceus().unwrap();
    team_1.members[0].species = "Arceus-Water".to_owned();
    team_1.members[0].item = Some("Splash Plate".to_owned());
    let mut battle = make_battle(0, team_1, kecleon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kecleon,player-2,1|name:Trick|noanim",
            "fail|mon:Kecleon,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn multitype_does_not_allow_plate_to_be_given() {
    let mut team_2 = kecleon().unwrap();
    team_2.members[0].item = Some("Splash Plate".to_owned());
    let mut battle = make_battle(0, arceus().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kecleon,player-2,1|name:Trick|noanim",
            "fail|mon:Kecleon,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
