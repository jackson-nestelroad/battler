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
                    "name": "Slurpuff",
                    "species": "Slurpuff",
                    "ability": "Sweet Veil",
                    "moves": [
                        "Sleep Powder",
                        "Yawn"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Slurpuff",
                    "species": "Slurpuff",
                    "ability": "No Ability",
                    "moves": [
                        "Rest"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "persistent_battle_data": {
                        "hp": 1
                    }
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
fn sweet_veil_prevents_user_and_ally_sleep() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Slurpuff,player-1,1|name:Sleep Powder|noanim",
            "block|mon:Slurpuff,player-2,1|move:Sleep Powder|from:ability:Sweet Veil",
            "fail|mon:Slurpuff,player-1,1",
            "move|mon:Slurpuff,player-1,2|name:Rest|noanim",
            "block|mon:Slurpuff,player-1,2|move:Rest|from:ability:Sweet Veil",
            "move|mon:Slurpuff,player-2,1|name:Yawn|noanim",
            "block|mon:Slurpuff,player-1,2|move:Yawn|from:ability:Sweet Veil",
            "fail|mon:Slurpuff,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
