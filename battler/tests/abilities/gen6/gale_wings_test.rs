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
                    "name": "Talonflame",
                    "species": "Talonflame",
                    "ability": "No Ability",
                    "moves": [
                        "Peck"
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
fn gale_wings_boosts_priority_at_full_health() {
    let mut team_1 = team().unwrap();
    team_1.members[0].ability = "Gale Wings".to_owned();
    let mut team_2 = team().unwrap();
    team_2.members[0].evs.spe = 252;
    let mut battle = make_battle(0, team_1, team_2).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Talonflame,player-1,1|name:Peck|target:Talonflame,player-2,1",
            "split|side:1",
            "damage|mon:Talonflame,player-2,1|health:217/266",
            "damage|mon:Talonflame,player-2,1|health:82/100",
            "move|mon:Talonflame,player-2,1|name:Peck|target:Talonflame,player-1,1",
            "split|side:0",
            "damage|mon:Talonflame,player-1,1|health:220/266",
            "damage|mon:Talonflame,player-1,1|health:83/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Talonflame,player-2,1|name:Peck|target:Talonflame,player-1,1",
            "split|side:0",
            "damage|mon:Talonflame,player-1,1|health:177/266",
            "damage|mon:Talonflame,player-1,1|health:67/100",
            "move|mon:Talonflame,player-1,1|name:Peck|target:Talonflame,player-2,1",
            "split|side:1",
            "damage|mon:Talonflame,player-2,1|health:171/266",
            "damage|mon:Talonflame,player-2,1|health:65/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
