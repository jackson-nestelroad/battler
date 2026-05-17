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

fn silvally() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Silvally",
                    "species": "Silvally",
                    "ability": "RKS System",
                    "moves": [
                        "Multi-Attack"
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
fn rks_system_changes_silvally_forme_dynamically_if_incorrect() {
    let mut team_1 = silvally().unwrap();
    team_1.members[0].item = Some("Fire Memory".to_owned());
    let mut team_2 = silvally().unwrap();
    team_2.members[0].item = Some("Water Memory".to_owned());
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Silvally"],
            ["switch", "player-1", "Silvally"],
            "split|side:1",
            ["switch", "player-2", "Silvally"],
            ["switch", "player-2", "Silvally"],
            "split|side:1",
            ["specieschange", "player-2", "species:Silvally-Water"],
            ["specieschange", "player-2", "species:Silvally-Water"],
            "formechange|mon:Silvally,player-2,1|species:Silvally-Water|from:ability:RKS System",
            "split|side:0",
            ["specieschange", "player-1", "species:Silvally-Fire"],
            ["specieschange", "player-1", "species:Silvally-Fire"],
            "formechange|mon:Silvally,player-1,1|species:Silvally-Fire|from:ability:RKS System",
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn rks_system_does_not_change_silvally_forme_if_correct() {
    let mut team_1 = silvally().unwrap();
    team_1.members[0].species = "Silvally-Fire".to_owned();
    team_1.members[0].item = Some("Fire Memory".to_owned());
    let mut team_2 = silvally().unwrap();
    team_2.members[0].species = "Silvally-Water".to_owned();
    team_2.members[0].item = Some("Water Memory".to_owned());
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Silvally-Fire"],
            ["switch", "player-1", "Silvally-Fire"],
            "split|side:1",
            ["switch", "player-2", "Silvally-Water"],
            ["switch", "player-2", "Silvally-Water"],
            "turn|turn:1"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn rks_system_works_for_non_silvally() {
    let mut team_1 = silvally().unwrap();
    team_1.members[0].species = "Silvally-Water".to_owned();
    team_1.members[0].item = Some("Water Memory".to_owned());
    let mut team_2 = kecleon().unwrap();
    team_2.members[0].ability = "RKS System".to_owned();
    team_2.members[0].item = Some("Water Memory".to_owned());
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Silvally,player-1,1|name:Multi-Attack|target:Kecleon,player-2,1",
            "resisted|mon:Kecleon,player-2,1",
            "split|side:1",
            "damage|mon:Kecleon,player-2,1|health:69/120",
            "damage|mon:Kecleon,player-2,1|health:58/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rks_system_does_not_change_types_when_transformed() {
    let mut team_1 = silvally().unwrap();
    team_1.members[0].species = "Silvally-Water".to_owned();
    team_1.members[0].item = Some("Water Memory".to_owned());
    let mut battle = make_battle(0, team_1, ditto().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-2,1|name:Transform|target:Silvally,player-1,1",
            "transform|mon:Ditto,player-2,1|into:Silvally,player-1,1|species:Silvally-Water",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Silvally,player-1,1|name:Multi-Attack|target:Ditto,player-2,1",
            "resisted|mon:Ditto,player-2,1",
            "split|side:1",
            "damage|mon:Ditto,player-2,1|health:69/108",
            "damage|mon:Ditto,player-2,1|health:64/100",
            "move|mon:Ditto,player-2,1|name:Multi-Attack|target:Silvally,player-1,1",
            "split|side:0",
            "damage|mon:Silvally,player-1,1|health:107/155",
            "damage|mon:Silvally,player-1,1|health:70/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn rks_system_does_not_allow_memory_to_be_taken() {
    let mut team_1 = silvally().unwrap();
    team_1.members[0].species = "Silvally-Water".to_owned();
    team_1.members[0].item = Some("Water Memory".to_owned());
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
fn rks_system_does_not_allow_memory_to_be_given() {
    let mut team_2 = kecleon().unwrap();
    team_2.members[0].item = Some("Water Memory".to_owned());
    let mut battle = make_battle(0, silvally().unwrap(), team_2).unwrap();
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
