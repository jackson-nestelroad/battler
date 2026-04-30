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
                    "name": "Comfey",
                    "species": "Comfey",
                    "ability": "No Ability",
                    "moves": [
                        "Floral Healing",
                        "Grassy Terrain"
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
fn floral_healing_heals_health_based_on_grassy_terrain() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Comfey,player-1,1|name:Floral Healing|target:Comfey,player-2,1",
            "split|side:1",
            "heal|mon:Comfey,player-2,1|health:107/212",
            "heal|mon:Comfey,player-2,1|health:51/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Comfey,player-1,1|name:Grassy Terrain",
            "fieldstart|move:Grassy Terrain",
            "move|mon:Comfey,player-2,1|name:Floral Healing|target:Comfey,player-1,1",
            "split|side:0",
            "heal|mon:Comfey,player-1,1|health:142/212",
            "heal|mon:Comfey,player-1,1|health:67/100",
            "split|side:0",
            "heal|mon:Comfey,player-1,1|from:move:Grassy Terrain|health:155/212",
            "heal|mon:Comfey,player-1,1|from:move:Grassy Terrain|health:74/100",
            "split|side:1",
            "heal|mon:Comfey,player-2,1|from:move:Grassy Terrain|health:120/212",
            "heal|mon:Comfey,player-2,1|from:move:Grassy Terrain|health:57/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
