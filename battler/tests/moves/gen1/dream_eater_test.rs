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

fn gengar() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gengar",
                    "species": "Gengar",
                    "ability": "No Ability",
                    "moves": [
                        "Dream Eater",
                        "Sleep Powder",
                        "Flamethrower"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
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
fn dream_eater_drains_damage_of_sleeping_foe() {
    let mut battle = make_battle(0, gengar().unwrap(), gengar().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-2,1|name:Flamethrower|target:Gengar,player-1,1",
            "split|side:0",
            "damage|mon:Gengar,player-1,1|health:55/120",
            "damage|mon:Gengar,player-1,1|health:46/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Gengar,player-1,1|name:Dream Eater|noanim",
            "immune|mon:Gengar,player-2,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Gengar,player-1,1|name:Sleep Powder|target:Gengar,player-2,1",
            "status|mon:Gengar,player-2,1|status:Sleep",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Gengar,player-1,1|name:Dream Eater|target:Gengar,player-2,1",
            "supereffective|mon:Gengar,player-2,1",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|health:0",
            "damage|mon:Gengar,player-2,1|health:0",
            "split|side:0",
            "heal|mon:Gengar,player-1,1|from:Drain|of:Gengar,player-2,1|health:115/120",
            "heal|mon:Gengar,player-1,1|from:Drain|of:Gengar,player-2,1|health:96/100",
            "faint|mon:Gengar,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
