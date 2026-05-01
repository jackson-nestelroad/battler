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
                    "name": "Ninetales",
                    "species": "Ninetales-Alola",
                    "ability": "No Ability",
                    "moves": [
                        "Aurora Veil",
                        "Snowscape",
                        "Light Screen",
                        "Flamethrower"
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
fn aurora_veil_activates_in_snow_and_does_not_stack_with_other_screens() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ninetales,player-1,1|name:Aurora Veil|noanim",
            "fail|mon:Ninetales,player-1,1",
            "move|mon:Ninetales,player-2,1|name:Snowscape",
            "weather|weather:Snow",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Ninetales,player-1,1|name:Aurora Veil",
            "sidestart|side:0|move:Aurora Veil",
            "move|mon:Ninetales,player-2,1|name:Flamethrower|target:Ninetales,player-1,1",
            "supereffective|mon:Ninetales,player-1,1",
            "split|side:0",
            "damage|mon:Ninetales,player-1,1|health:195/256",
            "damage|mon:Ninetales,player-1,1|health:77/100",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Ninetales,player-1,1|name:Light Screen",
            "sidestart|side:0|move:Light Screen",
            "move|mon:Ninetales,player-2,1|name:Flamethrower|target:Ninetales,player-1,1",
            "supereffective|mon:Ninetales,player-1,1",
            "split|side:0",
            "damage|mon:Ninetales,player-1,1|health:138/256",
            "damage|mon:Ninetales,player-1,1|health:54/100",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
