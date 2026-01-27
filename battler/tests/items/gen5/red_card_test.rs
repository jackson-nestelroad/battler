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
                    "name": "Snivy",
                    "species": "Snivy",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Red Card"
                },
                {
                    "name": "Tepig",
                    "species": "Tepig",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Red Card"
                },
                {
                    "name": "Oshawott",
                    "species": "Oshawott",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Red Card"
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
fn red_card_force_switches_when_hit() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snivy,player-1,1|name:Tackle|target:Snivy,player-2,1",
            "split|side:1",
            "damage|mon:Snivy,player-2,1|health:90/105",
            "damage|mon:Snivy,player-2,1|health:86/100",
            "itemend|mon:Snivy,player-2,1|item:Red Card",
            "split|side:0",
            ["drag", "player-1", "Tepig"],
            ["drag", "player-1", "Tepig"],
            "move|mon:Snivy,player-2,1|name:Tackle|target:Tepig,player-1,1",
            "split|side:0",
            "damage|mon:Tepig,player-1,1|health:108/125",
            "damage|mon:Tepig,player-1,1|health:87/100",
            "itemend|mon:Tepig,player-1,1|item:Red Card",
            "split|side:1",
            ["drag", "player-2", "Oshawott"],
            ["drag", "player-2", "Oshawott"],
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
