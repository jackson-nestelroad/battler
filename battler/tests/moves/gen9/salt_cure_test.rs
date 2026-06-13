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
                    "name": "Garganacl",
                    "species": "Baxcalibur",
                    "ability": "Sturdy",
                    "moves": [
                        "Salt Cure"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Gholdengo",
                    "species": "Gholdengo",
                    "ability": "No Ability",
                    "moves": [],
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
fn salt_cure_deals_residual_damage() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Garganacl,player-1,1|name:Salt Cure|target:Garganacl,player-2,1",
            "supereffective|mon:Garganacl,player-2,1",
            "split|side:1",
            "damage|mon:Garganacl,player-2,1|health:119/175",
            "damage|mon:Garganacl,player-2,1|health:68/100",
            "start|mon:Garganacl,player-2,1|move:Salt Cure",
            "move|mon:Garganacl,player-2,1|name:Salt Cure|target:Gholdengo,player-1,2",
            "resisted|mon:Gholdengo,player-1,2",
            "split|side:0",
            "damage|mon:Gholdengo,player-1,2|health:135/147",
            "damage|mon:Gholdengo,player-1,2|health:92/100",
            "start|mon:Gholdengo,player-1,2|move:Salt Cure",
            "split|side:1",
            "damage|mon:Garganacl,player-2,1|from:move:Salt Cure|health:98/175",
            "damage|mon:Garganacl,player-2,1|from:move:Salt Cure|health:56/100",
            "split|side:0",
            "damage|mon:Gholdengo,player-1,2|from:move:Salt Cure|health:99/147",
            "damage|mon:Gholdengo,player-1,2|from:move:Salt Cure|health:68/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
