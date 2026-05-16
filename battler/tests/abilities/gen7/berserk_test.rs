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
                    "name": "Drampa",
                    "species": "Drampa",
                    "ability": "Berserk",
                    "moves": [
                        "Tackle",
                        "Dragon Claw",
                        "Double Kick"
                    ],
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
fn berserk_boosts_special_attack_when_health_drops_below_half() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Drampa,player-1,1|name:Tackle|target:Drampa,player-2,1",
            "split|side:1",
            "damage|mon:Drampa,player-2,1|health:229/266",
            "damage|mon:Drampa,player-2,1|health:87/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Drampa,player-1,1|name:Dragon Claw|target:Drampa,player-2,1",
            "supereffective|mon:Drampa,player-2,1",
            "split|side:1",
            "damage|mon:Drampa,player-2,1|health:95/266",
            "damage|mon:Drampa,player-2,1|health:36/100",
            "boost|mon:Drampa,player-2,1|stat:spa|by:1|from:ability:Berserk",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn berserk_boosts_after_berry_for_single_hit_move() {
    let mut team_2 = team().unwrap();
    team_2.members[0].item = Some("Sitrus Berry".to_owned());
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Drampa,player-1,1|name:Dragon Claw|target:Drampa,player-2,1",
            "supereffective|mon:Drampa,player-2,1",
            "split|side:1",
            "damage|mon:Drampa,player-2,1|health:122/266",
            "damage|mon:Drampa,player-2,1|health:46/100",
            "boost|mon:Drampa,player-2,1|stat:spa|by:1|from:ability:Berserk",
            "itemend|mon:Drampa,player-2,1|item:Sitrus Berry|eat",
            "split|side:1",
            "heal|mon:Drampa,player-2,1|from:item:Sitrus Berry|health:188/266",
            "heal|mon:Drampa,player-2,1|from:item:Sitrus Berry|health:71/100",
            "residual",
            "turn|turn:2"
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

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Drampa,player-1,1|name:Double Kick|target:Drampa,player-2,1",
            "supereffective|mon:Drampa,player-2,1",
            "split|side:1",
            "damage|mon:Drampa,player-2,1|health:190/266",
            "damage|mon:Drampa,player-2,1|health:72/100",
            "animatemove|mon:Drampa,player-1,1|name:Double Kick|target:Drampa,player-2,1",
            "supereffective|mon:Drampa,player-2,1",
            "split|side:1",
            "damage|mon:Drampa,player-2,1|health:110/266",
            "damage|mon:Drampa,player-2,1|health:42/100",
            "itemend|mon:Drampa,player-2,1|item:Sitrus Berry|eat",
            "split|side:1",
            "heal|mon:Drampa,player-2,1|from:item:Sitrus Berry|health:176/266",
            "heal|mon:Drampa,player-2,1|from:item:Sitrus Berry|health:67/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Drampa,player-1,1|name:Double Kick|target:Drampa,player-2,1",
            "supereffective|mon:Drampa,player-2,1",
            "split|side:1",
            "damage|mon:Drampa,player-2,1|health:100/266",
            "damage|mon:Drampa,player-2,1|health:38/100",
            "animatemove|mon:Drampa,player-1,1|name:Double Kick|target:Drampa,player-2,1",
            "supereffective|mon:Drampa,player-2,1",
            "split|side:1",
            "damage|mon:Drampa,player-2,1|health:24/266",
            "damage|mon:Drampa,player-2,1|health:10/100",
            "hitcount|hits:2",
            "boost|mon:Drampa,player-2,1|stat:spa|by:1|from:ability:Berserk",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
