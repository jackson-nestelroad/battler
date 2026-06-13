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

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Great Tusk",
                    "species": "Great Tusk",
                    "ability": "Protosynthesis",
                    "moves": [
                        "Sunny Day",
                        "Rain Dance"
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
fn protosynthesis_boosts_best_stat_in_sunlight() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Great Tusk,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "activate|mon:Great Tusk,player-1,1|ability:Protosynthesis",
            "start|mon:Great Tusk,player-1,1|ability:Protosynthesis|stat:atk",
            "activate|mon:Great Tusk,player-2,1|ability:Protosynthesis",
            "start|mon:Great Tusk,player-2,1|ability:Protosynthesis|stat:atk",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Great Tusk,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "end|mon:Great Tusk,player-1,1|ability:Protosynthesis",
            "end|mon:Great Tusk,player-2,1|ability:Protosynthesis",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn booster_energy_starts_protosynthesis_regardless_of_weather() {
    let mut team_1 = team().unwrap();
    team_1.members[0].item = Some("Booster Energy".to_owned());
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Great Tusk"],
            ["switch", "player-1", "Great Tusk"],
            "split|side:1",
            ["switch", "player-2", "Great Tusk"],
            ["switch", "player-2", "Great Tusk"],
            "itemend|mon:Great Tusk,player-1,1|item:Booster Energy",
            "activate|mon:Great Tusk,player-1,1|ability:Protosynthesis|from:item:Booster Energy",
            "start|mon:Great Tusk,player-1,1|ability:Protosynthesis|stat:atk",
            "turn|turn:1",
            "continue",
            "move|mon:Great Tusk,player-1,1|name:Sunny Day",
            "weather|weather:Harsh Sunlight",
            "activate|mon:Great Tusk,player-2,1|ability:Protosynthesis",
            "start|mon:Great Tusk,player-2,1|ability:Protosynthesis|stat:atk",
            "weather|weather:Harsh Sunlight|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Great Tusk,player-1,1|name:Rain Dance",
            "weather|weather:Rain",
            "end|mon:Great Tusk,player-2,1|ability:Protosynthesis",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
