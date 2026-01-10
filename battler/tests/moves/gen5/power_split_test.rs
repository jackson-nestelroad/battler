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

fn shuckle() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Shuckle",
                    "species": "Shuckle",
                    "ability": "Sturdy",
                    "moves": [
                        "Power Split",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn machamp() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Machamp",
                    "species": "Machamp",
                    "ability": "No Guard",
                    "moves": [
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "evs": {
                        "atk": 252
                    }
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
fn power_split_averages_attack() {
    let mut battle = make_battle(0, shuckle().unwrap(), machamp().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1
    // Shuckle uses Tackle. Low damage.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2
    // Shuckle uses Power Split. Averages stats.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3
    // Shuckle uses Tackle again. Higher damage.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Shuckle,player-1,1|name:Tackle|target:Machamp,player-2,1",
            "crit|mon:Machamp,player-2,1",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:144/150",
            "damage|mon:Machamp,player-2,1|health:96/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Shuckle,player-1,1|name:Power Split|target:Machamp,player-2,1",
            "activate|mon:Machamp,player-2,1|move:Power Split|of:Shuckle,player-1,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Shuckle,player-1,1|name:Tackle|target:Machamp,player-2,1",
            "split|side:1",
            "damage|mon:Machamp,player-2,1|health:125/150",
            "damage|mon:Machamp,player-2,1|health:84/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
