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

fn woobat() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Woobat",
                    "species": "Woobat",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
    .unwrap()
}

fn make_battle(
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>, anyhow::Error> {
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
fn simple_beam_changes_ability_to_simple() {
    let mut user = woobat();
    user.members[0].moves = vec!["Simple Beam".to_owned()];
    let mut target = woobat();
    target.members[0].ability = "Klutz".to_owned();
    target.members[0].moves = vec!["Defense Curl".to_owned()];
    let mut battle = make_battle(BattleType::Singles, 0, user, target).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Woobat,player-1,1|name:Simple Beam|target:Woobat,player-2,1",
            "abilityend|mon:Woobat,player-2,1|ability:Klutz|from:move:Simple Beam|of:Woobat,player-1,1",
            "ability|mon:Woobat,player-2,1|ability:Simple|from:move:Simple Beam|of:Woobat,player-1,1",
            "move|mon:Woobat,player-2,1|name:Defense Curl|target:Woobat,player-2,1",
            "boost|mon:Woobat,player-2,1|stat:def|by:2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn simple_beam_fails_against_specific_abilities() {
    let mut user = woobat();
    user.members[0].moves = vec!["Simple Beam".to_owned()];
    let mut target = woobat();
    target.members[0].ability = "Truant".to_owned();
    let mut battle = make_battle(BattleType::Singles, 0, user, target).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Woobat,player-1,1|name:Simple Beam|noanim",
            "fail|mon:Woobat,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn simple_beam_fails_against_simple() {
    let mut user = woobat();
    user.members[0].moves = vec!["Simple Beam".to_owned()];
    let mut target = woobat();
    target.members[0].ability = "Simple".to_owned();
    let mut battle = make_battle(BattleType::Singles, 0, user, target).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Woobat,player-1,1|name:Simple Beam|noanim",
            "fail|mon:Woobat,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
