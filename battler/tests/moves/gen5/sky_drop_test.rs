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
                    "name": "Hawlucha",
                    "species": "Hawlucha",
                    "ability": "No Ability",
                    "moves": [
                        "Sky Drop",
                        "Earthquake",
                        "Gust"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Samurott",
                    "species": "Samurott",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Follow Me"
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
fn sky_drop_lifts_target_into_air_and_damages_on_next_turn() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;pass"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Hawlucha does not have a move in slot 1")
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-1,1|move:Sky Drop|target:Samurott,player-2,2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|target:Samurott,player-2,2",
            "end|mon:Samurott,player-2,2|move:Sky Drop",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:112/155",
            "damage|mon:Samurott,player-2,2|health:73/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sky_drop_prevents_target_from_moving() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-1,1|move:Sky Drop|target:Samurott,player-2,2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|target:Samurott,player-2,2",
            "end|mon:Samurott,player-2,2|move:Sky Drop",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:112/155",
            "damage|mon:Samurott,player-2,2|health:73/100",
            "move|mon:Samurott,player-2,2|name:Tackle|target:Samurott,player-1,2",
            "split|side:0",
            "damage|mon:Samurott,player-1,2|health:136/155",
            "damage|mon:Samurott,player-1,2|health:88/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sky_drop_makes_target_and_user_invulnerable() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-1,1|move:Sky Drop|target:Samurott,player-2,2",
            "move|mon:Hawlucha,player-2,1|name:Earthquake|spread:Samurott,player-1,2",
            "miss|mon:Samurott,player-2,2",
            "miss|mon:Hawlucha,player-1,1",
            "split|side:0",
            "damage|mon:Samurott,player-1,2|health:120/155",
            "damage|mon:Samurott,player-1,2|health:78/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|target:Samurott,player-2,2",
            "end|mon:Samurott,player-2,2|move:Sky Drop",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:115/155",
            "damage|mon:Samurott,player-2,2|health:75/100",
            "move|mon:Hawlucha,player-2,1|name:Earthquake|spread:Samurott,player-2,2;Samurott,player-1,2",
            "immune|mon:Hawlucha,player-1,1",
            "split|side:1",
            "damage|mon:Samurott,player-2,2|health:79/155",
            "damage|mon:Samurott,player-2,2|health:51/100",
            "split|side:0",
            "damage|mon:Samurott,player-1,2|health:87/155",
            "damage|mon:Samurott,player-1,2|health:57/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sky_drop_target_and_user_vulnerable_to_gust() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-1,1|move:Sky Drop|target:Samurott,player-2,2",
            "move|mon:Hawlucha,player-2,1|name:Gust|target:Hawlucha,player-1,1",
            "supereffective|mon:Hawlucha,player-1,1",
            "split|side:0",
            "damage|mon:Hawlucha,player-1,1|health:14/138",
            "damage|mon:Hawlucha,player-1,1|health:11/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sky_drop_canceled_if_user_faints_in_air() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2,1;move 0,2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-1,1|move:Sky Drop|target:Samurott,player-2,2",
            "move|mon:Hawlucha,player-2,1|name:Gust|target:Hawlucha,player-1,1",
            "supereffective|mon:Hawlucha,player-1,1",
            "split|side:0",
            "damage|mon:Hawlucha,player-1,1|health:0",
            "damage|mon:Hawlucha,player-1,1|health:0",
            "faint|mon:Hawlucha,player-1,1",
            "end|mon:Samurott,player-2,2|move:Sky Drop",
            "move|mon:Samurott,player-2,2|name:Tackle|target:Samurott,player-1,2",
            "split|side:0",
            "damage|mon:Samurott,player-1,2|health:137/155",
            "damage|mon:Samurott,player-1,2|health:89/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}

#[test]
fn sky_drop_fails_on_second_turn_for_flying_type_target() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-1,1|move:Sky Drop|target:Hawlucha,player-2,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|noanim",
            "immune|mon:Hawlucha,player-2,1",
            "end|mon:Hawlucha,player-2,1|move:Sky Drop",
            "move|mon:Samurott,player-2,2|name:Tackle|target:Hawlucha,player-1,1",
            "split|side:0",
            "damage|mon:Hawlucha,player-1,1|health:114/138",
            "damage|mon:Hawlucha,player-1,1|health:83/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn follow_me_fails_during_sky_drop() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Samurott,player-2,2|name:Follow Me|target:Samurott,player-2,2",
            "singleturn|mon:Samurott,player-2,2|move:Follow Me",
            "move|mon:Hawlucha,player-1,1|name:Sky Drop|noanim",
            "prepare|mon:Hawlucha,player-1,1|move:Sky Drop|target:Samurott,player-2,2",
            "move|mon:Samurott,player-1,2|name:Tackle|target:Hawlucha,player-2,1",
            "split|side:1",
            "damage|mon:Hawlucha,player-2,1|health:114/138",
            "damage|mon:Hawlucha,player-2,1|health:83/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
