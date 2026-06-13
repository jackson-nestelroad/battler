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
                    "name": "Klawf",
                    "species": "Klawf",
                    "ability": "Anger Shell",
                    "moves": [
                        "Brick Break",
                        "Double Kick"
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
fn anger_shell_boosts_stats_when_health_drops_below_half() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Klawf,player-1,1|name:Brick Break|target:Klawf,player-2,1",
            "supereffective|mon:Klawf,player-2,1",
            "split|side:1",
            "damage|mon:Klawf,player-2,1|health:72/130",
            "damage|mon:Klawf,player-2,1|health:56/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Klawf,player-1,1|name:Brick Break|target:Klawf,player-2,1",
            "supereffective|mon:Klawf,player-2,1",
            "split|side:1",
            "damage|mon:Klawf,player-2,1|health:18/130",
            "damage|mon:Klawf,player-2,1|health:14/100",
            "boost|mon:Klawf,player-2,1|stat:atk|by:1|from:ability:Anger Shell",
            "unboost|mon:Klawf,player-2,1|stat:def|by:1|from:ability:Anger Shell",
            "boost|mon:Klawf,player-2,1|stat:spa|by:1|from:ability:Anger Shell",
            "unboost|mon:Klawf,player-2,1|stat:spd|by:1|from:ability:Anger Shell",
            "boost|mon:Klawf,player-2,1|stat:spe|by:1|from:ability:Anger Shell",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn anger_shell_boosts_before_berry_for_single_hit_move() {
    let mut team_2 = team().unwrap();
    team_2.members[0].item = Some("Sitrus Berry".to_owned());
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Klawf,player-1,1|name:Brick Break|target:Klawf,player-2,1",
            "supereffective|mon:Klawf,player-2,1",
            "split|side:1",
            "damage|mon:Klawf,player-2,1|health:72/130",
            "damage|mon:Klawf,player-2,1|health:56/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Klawf,player-1,1|name:Brick Break|target:Klawf,player-2,1",
            "supereffective|mon:Klawf,player-2,1",
            "split|side:1",
            "damage|mon:Klawf,player-2,1|health:18/130",
            "damage|mon:Klawf,player-2,1|health:14/100",
            "boost|mon:Klawf,player-2,1|stat:atk|by:1|from:ability:Anger Shell",
            "unboost|mon:Klawf,player-2,1|stat:def|by:1|from:ability:Anger Shell",
            "boost|mon:Klawf,player-2,1|stat:spa|by:1|from:ability:Anger Shell",
            "unboost|mon:Klawf,player-2,1|stat:spd|by:1|from:ability:Anger Shell",
            "boost|mon:Klawf,player-2,1|stat:spe|by:1|from:ability:Anger Shell",
            "itemend|mon:Klawf,player-2,1|item:Sitrus Berry|eat",
            "split|side:1",
            "heal|mon:Klawf,player-2,1|from:item:Sitrus Berry|health:50/130",
            "heal|mon:Klawf,player-2,1|from:item:Sitrus Berry|health:39/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn berserk_boosts_after_berry_for_multi_hit_move() {
    let mut team_2 = team().unwrap();
    team_2.members[0].item = Some("Sitrus Berry".to_owned());
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Klawf,player-1,1|name:Double Kick|target:Klawf,player-2,1",
            "supereffective|mon:Klawf,player-2,1",
            "split|side:1",
            "damage|mon:Klawf,player-2,1|health:106/130",
            "damage|mon:Klawf,player-2,1|health:82/100",
            "animatemove|mon:Klawf,player-1,1|name:Double Kick|target:Klawf,player-2,1",
            "supereffective|mon:Klawf,player-2,1",
            "split|side:1",
            "damage|mon:Klawf,player-2,1|health:80/130",
            "damage|mon:Klawf,player-2,1|health:62/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Klawf,player-1,1|name:Double Kick|target:Klawf,player-2,1",
            "supereffective|mon:Klawf,player-2,1",
            "split|side:1",
            "damage|mon:Klawf,player-2,1|health:56/130",
            "damage|mon:Klawf,player-2,1|health:44/100",
            "itemend|mon:Klawf,player-2,1|item:Sitrus Berry|eat",
            "split|side:1",
            "heal|mon:Klawf,player-2,1|from:item:Sitrus Berry|health:88/130",
            "heal|mon:Klawf,player-2,1|from:item:Sitrus Berry|health:68/100",
            "animatemove|mon:Klawf,player-1,1|name:Double Kick|target:Klawf,player-2,1",
            "supereffective|mon:Klawf,player-2,1",
            "split|side:1",
            "damage|mon:Klawf,player-2,1|health:64/130",
            "damage|mon:Klawf,player-2,1|health:50/100",
            "hitcount|hits:2",
            "boost|mon:Klawf,player-2,1|stat:atk|by:1|from:ability:Anger Shell",
            "unboost|mon:Klawf,player-2,1|stat:def|by:1|from:ability:Anger Shell",
            "boost|mon:Klawf,player-2,1|stat:spa|by:1|from:ability:Anger Shell",
            "unboost|mon:Klawf,player-2,1|stat:spd|by:1|from:ability:Anger Shell",
            "boost|mon:Klawf,player-2,1|stat:spe|by:1|from:ability:Anger Shell",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
