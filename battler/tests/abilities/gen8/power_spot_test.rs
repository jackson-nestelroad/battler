use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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
                    "name": "Stonjourner",
                    "species": "Stonjourner",
                    "ability": "No Ability",
                    "moves": [
                        "Crunch"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Stonjourner",
                    "species": "Stonjourner",
                    "ability": "No Ability",
                    "moves": [
                        "Crunch"
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn power_spot_boosts_power_of_ally_moves() {
    let mut team_1 = team().unwrap();
    team_1.members[0].ability = "Power Spot".to_owned();
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 0,2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Stonjourner,player-1,1|name:Crunch|target:Stonjourner,player-2,1",
            "split|side:1",
            "damage|mon:Stonjourner,player-2,1|health:246/310",
            "damage|mon:Stonjourner,player-2,1|health:80/100",
            "move|mon:Stonjourner,player-1,2|name:Crunch|target:Stonjourner,player-2,2",
            "split|side:1",
            "damage|mon:Stonjourner,player-2,2|health:227/310",
            "damage|mon:Stonjourner,player-2,2|health:74/100",
            "move|mon:Stonjourner,player-2,1|name:Crunch|target:Stonjourner,player-1,1",
            "split|side:0",
            "damage|mon:Stonjourner,player-1,1|health:246/310",
            "damage|mon:Stonjourner,player-1,1|health:80/100",
            "move|mon:Stonjourner,player-2,2|name:Crunch|target:Stonjourner,player-1,2",
            "split|side:0",
            "damage|mon:Stonjourner,player-1,2|health:246/310",
            "damage|mon:Stonjourner,player-1,2|health:80/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
