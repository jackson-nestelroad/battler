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

fn zigzagoon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Zigzagoon",
                    "species": "Zigzagoon",
                    "ability": "No Ability",
                    "moves": [
                        "Charge",
                        "Shock Wave"
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
fn charge_powers_up_next_electric_move() {
    let mut battle = make_battle(100, zigzagoon().unwrap(), zigzagoon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Zigzagoon,player-1,1|name:Shock Wave|target:Zigzagoon,player-2,1",
            "split|side:1",
            "damage|mon:Zigzagoon,player-2,1|health:80/98",
            "damage|mon:Zigzagoon,player-2,1|health:82/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Zigzagoon,player-1,1|name:Charge|target:Zigzagoon,player-1,1",
            "start|mon:Zigzagoon,player-1,1|move:Charge",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Zigzagoon,player-1,1|name:Shock Wave|target:Zigzagoon,player-2,1",
            "split|side:1",
            "damage|mon:Zigzagoon,player-2,1|health:44/98",
            "damage|mon:Zigzagoon,player-2,1|health:45/100",
            "end|mon:Zigzagoon,player-1,1|move:Charge|silent",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
