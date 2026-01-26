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

fn darumaka() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Darumaka",
                    "species": "Darumaka",
                    "ability": "Zen Mode",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn darmanitan() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Darmanitan",
                    "species": "Darmanitan",
                    "ability": "Zen Mode",
                    "moves": [
                        "Flamethrower",
                        "Surf",
                        "Recover",
                        "Gastro Acid"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn darmanitan_galar() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Darmanitan",
                    "species": "Darmanitan-Galar",
                    "ability": "Zen Mode",
                    "moves": [
                        "Recover"
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
fn zen_mode_does_nothing_for_non_darmanitan() {
    let mut battle = make_battle(0, darmanitan().unwrap(), darumaka().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darmanitan,player-1,1|name:Surf",
            "supereffective|mon:Darumaka,player-2,1",
            "split|side:1",
            "damage|mon:Darumaka,player-2,1|health:74/130",
            "damage|mon:Darumaka,player-2,1|health:57/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Darmanitan,player-1,1|name:Surf",
            "supereffective|mon:Darumaka,player-2,1",
            "split|side:1",
            "damage|mon:Darumaka,player-2,1|health:22/130",
            "damage|mon:Darumaka,player-2,1|health:17/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn zen_mode_transforms_darmanitan_at_end_of_turn() {
    let mut battle = make_battle(0, darmanitan().unwrap(), darmanitan().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darmanitan,player-1,1|name:Surf",
            "supereffective|mon:Darmanitan,player-2,1",
            "split|side:1",
            "damage|mon:Darmanitan,player-2,1|health:117/165",
            "damage|mon:Darmanitan,player-2,1|health:71/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Darmanitan,player-1,1|name:Surf",
            "supereffective|mon:Darmanitan,player-2,1",
            "split|side:1",
            "damage|mon:Darmanitan,player-2,1|health:73/165",
            "damage|mon:Darmanitan,player-2,1|health:45/100",
            "formechange|mon:Darmanitan,player-2,1|species:Darmanitan-Zen|from:ability:Zen Mode",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Darmanitan,player-2,1|name:Recover|target:Darmanitan,player-2,1",
            "split|side:1",
            "heal|mon:Darmanitan,player-2,1|health:156/165",
            "heal|mon:Darmanitan,player-2,1|health:95/100",
            "formechange|mon:Darmanitan,player-2,1|species:Darmanitan|from:ability:Zen Mode",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn zen_mode_transforms_darmanitan_galar_at_end_of_turn() {
    let mut battle = make_battle(0, darmanitan().unwrap(), darmanitan_galar().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darmanitan,player-1,1|name:Flamethrower|target:Darmanitan,player-2,1",
            "supereffective|mon:Darmanitan,player-2,1",
            "split|side:1",
            "damage|mon:Darmanitan,player-2,1|health:93/165",
            "damage|mon:Darmanitan,player-2,1|health:57/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Darmanitan,player-1,1|name:Flamethrower|target:Darmanitan,player-2,1",
            "supereffective|mon:Darmanitan,player-2,1",
            "split|side:1",
            "damage|mon:Darmanitan,player-2,1|health:27/165",
            "damage|mon:Darmanitan,player-2,1|health:17/100",
            "formechange|mon:Darmanitan,player-2,1|species:Darmanitan-Galar-Zen|from:ability:Zen Mode",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Darmanitan,player-2,1|name:Recover|target:Darmanitan,player-2,1",
            "split|side:1",
            "heal|mon:Darmanitan,player-2,1|health:110/165",
            "heal|mon:Darmanitan,player-2,1|health:67/100",
            "formechange|mon:Darmanitan,player-2,1|species:Darmanitan-Galar|from:ability:Zen Mode",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
