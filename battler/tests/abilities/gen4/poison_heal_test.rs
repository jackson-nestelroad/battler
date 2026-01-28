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

fn gliscor() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gliscor",
                    "species": "Gliscor",
                    "ability": "Poison Heal",
                    "item": "Toxic Orb",
                    "moves": [
                        "Tackle"
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
fn poison_heal_heals_each_turn_when_poisoned() {
    let mut battle = make_battle(0, gliscor().unwrap(), gliscor().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gliscor,player-1,1|name:Tackle|target:Gliscor,player-2,1",
            "split|side:1",
            "damage|mon:Gliscor,player-2,1|health:121/135",
            "damage|mon:Gliscor,player-2,1|health:90/100",
            "status|mon:Gliscor,player-1,1|status:Bad Poison|from:item:Toxic Orb",
            "status|mon:Gliscor,player-2,1|status:Bad Poison|from:item:Toxic Orb",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Gliscor,player-1,1|name:Tackle|target:Gliscor,player-2,1",
            "split|side:1",
            "damage|mon:Gliscor,player-2,1|health:108/135",
            "damage|mon:Gliscor,player-2,1|health:80/100",
            "split|side:1",
            "heal|mon:Gliscor,player-2,1|from:ability:Poison Heal|health:124/135",
            "heal|mon:Gliscor,player-2,1|from:ability:Poison Heal|health:92/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
