use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Skitty",
                    "species": "Skitty",
                    "ability": "No Ability",
                    "moves": [
                        "Assist"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Zigzagoon",
                    "species": "Zigzagoon",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Quick Attack",
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

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
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
        .build(data)
}

#[test]
fn assist_uses_random_move_from_side() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 25245345, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Skitty,player-1,1|name:Assist|target:Skitty,player-1,1",
            "move|mon:Skitty,player-1,1|name:Tackle|target:Zigzagoon,player-2,2|from:move:Assist",
            "split|side:1",
            "damage|mon:Zigzagoon,player-2,2|health:70/98",
            "damage|mon:Zigzagoon,player-2,2|health:72/100",
            "move|mon:Skitty,player-2,1|name:Assist|target:Skitty,player-2,1",
            "move|mon:Skitty,player-2,1|name:Quick Attack|target:Zigzagoon,player-1,2|from:move:Assist",
            "split|side:0",
            "damage|mon:Zigzagoon,player-1,2|health:71/98",
            "damage|mon:Zigzagoon,player-1,2|health:73/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
