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
                    "name": "Ribombee",
                    "species": "Ribombee",
                    "ability": "No Ability",
                    "moves": [
                        "Pollen Puff"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "persistent_battle_data": {
                        "hp": 1
                    }
                },
                {
                    "name": "Ribombee",
                    "species": "Ribombee",
                    "ability": "No Ability",
                    "moves": [
                        "Pollen Puff"
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
fn pollen_puff_damages_foe_and_heals_ally() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ribombee,player-1,1|name:Pollen Puff|target:Ribombee,player-2,2",
            "resisted|mon:Ribombee,player-2,2",
            "split|side:1",
            "damage|mon:Ribombee,player-2,2|health:156/230",
            "damage|mon:Ribombee,player-2,2|health:68/100",
            "move|mon:Ribombee,player-1,2|name:Pollen Puff|target:Ribombee,player-1,1",
            "split|side:0",
            "heal|mon:Ribombee,player-1,1|health:116/230",
            "heal|mon:Ribombee,player-1,1|health:51/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
