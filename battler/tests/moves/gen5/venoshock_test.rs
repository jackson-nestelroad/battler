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
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "Natural Cure",
                    "moves": [
                        "Venoshock",
                        "Toxic"
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
fn venoshock_doubles_power_against_poisoned_target() {
    let team_1 = team().unwrap();
    let team_2 = team().unwrap();
    let mut battle = make_battle(0, team_1, team_2).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Turn 1: Venoshock ~17 damage.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 2: Toxic.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    // Turn 3: Venoshock ~30 damage.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Blissey,player-1,1|name:Venoshock|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:298/315",
            "damage|mon:Blissey,player-2,1|health:95/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Blissey,player-1,1|name:Toxic|target:Blissey,player-2,1",
            "status|mon:Blissey,player-2,1|status:Bad Poison",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|from:status:Bad Poison|health:279/315",
            "damage|mon:Blissey,player-2,1|from:status:Bad Poison|health:89/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Blissey,player-1,1|name:Venoshock|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:249/315",
            "damage|mon:Blissey,player-2,1|health:80/100",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|from:status:Bad Poison|health:210/315",
            "damage|mon:Blissey,player-2,1|from:status:Bad Poison|health:67/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
