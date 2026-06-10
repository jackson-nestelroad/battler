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
                    "name": "Tauros",
                    "species": "Tauros",
                    "ability": "Color Change",
                    "moves": [
                        "Raging Bull"
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
fn raging_bull_changes_type_based_on_tauros_forme() {
    let mut team_1 = team().unwrap();
    team_1.members[0].species = "Tauros-Paldea-Blaze-Breed".to_owned();
    let mut team_2 = team().unwrap();
    team_2.members[0].species = "Tauros-Paldea-Aqua-Breed".to_owned();
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Tauros,player-1,1|name:Raging Bull|target:Tauros,player-2,1",
            "resisted|mon:Tauros,player-2,1",
            "split|side:1",
            "damage|mon:Tauros,player-2,1|health:202/260",
            "damage|mon:Tauros,player-2,1|health:78/100",
            "typechange|mon:Tauros,player-2,1|types:Fire|from:ability:Color Change",
            "move|mon:Tauros,player-2,1|name:Raging Bull|target:Tauros,player-1,1",
            "supereffective|mon:Tauros,player-1,1",
            "split|side:0",
            "damage|mon:Tauros,player-1,1|health:116/260",
            "damage|mon:Tauros,player-1,1|health:45/100",
            "typechange|mon:Tauros,player-1,1|types:Water|from:ability:Color Change",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
