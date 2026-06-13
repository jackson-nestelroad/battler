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
                    "name": "Grafaiai",
                    "species": "Grafaiai",
                    "ability": "No Ability",
                    "moves": [
                        "Doodle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Grafaiai",
                    "species": "Grafaiai",
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

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
fn doodle_copies_target_ability_to_user_and_allies() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Speed Boost".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grafaiai,player-1,1|name:Doodle|target:Grafaiai,player-2,1",
            "abilityend|mon:Grafaiai,player-1,1|ability:No Ability|from:move:Doodle",
            "ability|mon:Grafaiai,player-1,1|ability:Speed Boost|from:move:Doodle",
            "abilityend|mon:Grafaiai,player-1,2|ability:No Ability|from:move:Doodle|of:Grafaiai,player-1,1",
            "ability|mon:Grafaiai,player-1,2|ability:Speed Boost|from:move:Doodle|of:Grafaiai,player-1,1",
            "boost|mon:Grafaiai,player-2,1|stat:spe|by:1|from:ability:Speed Boost",
            "boost|mon:Grafaiai,player-1,1|stat:spe|by:1|from:ability:Speed Boost",
            "boost|mon:Grafaiai,player-1,2|stat:spe|by:1|from:ability:Speed Boost",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn doodle_does_not_overwrite_permanent_ability() {
    let mut team_1 = team().unwrap();
    team_1.members[1].ability = "As One".to_owned();
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Speed Boost".to_owned();
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grafaiai,player-1,1|name:Doodle|target:Grafaiai,player-2,1",
            "abilityend|mon:Grafaiai,player-1,1|ability:No Ability|from:move:Doodle",
            "ability|mon:Grafaiai,player-1,1|ability:Speed Boost|from:move:Doodle",
            "boost|mon:Grafaiai,player-2,1|stat:spe|by:1|from:ability:Speed Boost",
            "boost|mon:Grafaiai,player-1,1|stat:spe|by:1|from:ability:Speed Boost",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn doodle_fails_if_abilities_cannot_be_set() {
    let mut team_1 = team().unwrap();
    team_1.members[0].item = Some("Ability Shield".to_owned());
    team_1.members[1].item = Some("Ability Shield".to_owned());
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Speed Boost".to_owned();
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grafaiai,player-1,1|name:Doodle|noanim",
            "block|mon:Grafaiai,player-1,1|move:Doodle|from:item:Ability Shield",
            "block|mon:Grafaiai,player-1,2|move:Doodle|from:item:Ability Shield",
            "boost|mon:Grafaiai,player-2,1|stat:spe|by:1|from:ability:Speed Boost",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn doodle_fails_if_target_ability_cannot_be_copied() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Teraform Zero".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grafaiai,player-1,1|name:Doodle|noanim",
            "fail|mon:Grafaiai,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn doodle_fails_if_all_ally_abilities_cannot_be_changed() {
    let mut team_1 = team().unwrap();
    team_1.members[0].ability = "Tera Shift".to_owned();
    team_1.members[1].ability = "Tera Shift".to_owned();
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "Speed Boost".to_owned();
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Grafaiai,player-1,1|name:Doodle|noanim",
            "fail|mon:Grafaiai,player-1,1",
            "boost|mon:Grafaiai,player-2,1|stat:spe|by:1|from:ability:Speed Boost",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
