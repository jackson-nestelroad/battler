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
                    "name": "Falinks",
                    "species": "Falinks",
                    "ability": "No Ability",
                    "moves": [
                        "No Retreat",
                        "Trick-or-Treat",
                        "Mean Look"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Falinks",
                    "species": "Falinks",
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
fn no_retreat_boosts_stats_traps_user() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: Falinks is trapped", "{err:?}")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Falinks,player-1,1|name:No Retreat|target:Falinks,player-1,1",
            "boost|mon:Falinks,player-1,1|stat:atk|by:1",
            "boost|mon:Falinks,player-1,1|stat:def|by:1",
            "boost|mon:Falinks,player-1,1|stat:spa|by:1",
            "boost|mon:Falinks,player-1,1|stat:spd|by:1",
            "boost|mon:Falinks,player-1,1|stat:spe|by:1",
            "start|mon:Falinks,player-1,1|move:No Retreat",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Falinks,player-1,1|name:No Retreat|noanim",
            "fail|mon:Falinks,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn no_retreat_can_only_be_used_once_by_ghost_type() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Falinks,player-2,1|name:Trick-or-Treat|target:Falinks,player-1,1",
            "addedtype|mon:Falinks,player-1,1|type:Ghost",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Falinks,player-1,1|name:No Retreat|target:Falinks,player-1,1",
            "boost|mon:Falinks,player-1,1|stat:atk|by:1",
            "boost|mon:Falinks,player-1,1|stat:def|by:1",
            "boost|mon:Falinks,player-1,1|stat:spa|by:1",
            "boost|mon:Falinks,player-1,1|stat:spd|by:1",
            "boost|mon:Falinks,player-1,1|stat:spe|by:1",
            "start|mon:Falinks,player-1,1|move:No Retreat",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Falinks,player-1,1|name:No Retreat|noanim",
            "fail|mon:Falinks,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn no_retreat_can_be_used_multiple_times_if_already_trapped() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Falinks,player-2,1|name:Mean Look|target:Falinks,player-1,1",
            "activate|mon:Falinks,player-1,1|condition:Trapped",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Falinks,player-1,1|name:No Retreat|target:Falinks,player-1,1",
            "boost|mon:Falinks,player-1,1|stat:atk|by:1",
            "boost|mon:Falinks,player-1,1|stat:def|by:1",
            "boost|mon:Falinks,player-1,1|stat:spa|by:1",
            "boost|mon:Falinks,player-1,1|stat:spd|by:1",
            "boost|mon:Falinks,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Falinks,player-1,1|name:No Retreat|target:Falinks,player-1,1",
            "boost|mon:Falinks,player-1,1|stat:atk|by:1",
            "boost|mon:Falinks,player-1,1|stat:def|by:1",
            "boost|mon:Falinks,player-1,1|stat:spa|by:1",
            "boost|mon:Falinks,player-1,1|stat:spd|by:1",
            "boost|mon:Falinks,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
