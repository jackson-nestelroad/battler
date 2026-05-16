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
                    "name": "Beartic",
                    "species": "Beartic",
                    "ability": "Slush Rush",
                    "moves": [
                        "Snowscape",
                        "Agility",
                        "Tackle"
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
fn slush_rush_boosts_speed_in_snow() {
    let mut team_2 = team().unwrap();
    team_2.members[0].ability = "No Ability".to_owned();
    let mut battle = make_battle(0, team().unwrap(), team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Beartic,player-1,1|name:Snowscape",
            "weather|weather:Snow",
            "move|mon:Beartic,player-2,1|name:Agility|target:Beartic,player-2,1",
            "boost|mon:Beartic,player-2,1|stat:spe|by:2",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Beartic,player-1,1|name:Tackle|target:Beartic,player-2,1",
            "split|side:1",
            "damage|mon:Beartic,player-2,1|health:264/300",
            "damage|mon:Beartic,player-2,1|health:88/100",
            "move|mon:Beartic,player-2,1|name:Tackle|target:Beartic,player-1,1",
            "split|side:0",
            "damage|mon:Beartic,player-1,1|health:266/300",
            "damage|mon:Beartic,player-1,1|health:89/100",
            "weather|weather:Snow|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
