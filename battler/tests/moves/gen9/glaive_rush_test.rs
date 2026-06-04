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
                    "name": "Baxcalibur",
                    "species": "Baxcalibur",
                    "ability": "Sturdy",
                    "moves": [
                        "Glaive Rush",
                        "Flamethrower",
                        "Recover"
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
fn glaive_rush_forces_user_to_take_double_damage_until_next_move() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Baxcalibur,player-1,1|name:Flamethrower|target:Baxcalibur,player-2,1",
            "split|side:1",
            "damage|mon:Baxcalibur,player-2,1|health:141/175",
            "damage|mon:Baxcalibur,player-2,1|health:81/100",
            "move|mon:Baxcalibur,player-2,1|name:Glaive Rush|target:Baxcalibur,player-1,1",
            "supereffective|mon:Baxcalibur,player-1,1",
            "activate|mon:Baxcalibur,player-1,1|ability:Sturdy",
            "split|side:0",
            "damage|mon:Baxcalibur,player-1,1|health:1/175",
            "damage|mon:Baxcalibur,player-1,1|health:1/100",
            "singlemove|mon:Baxcalibur,player-2,1|move:Glaive Rush",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Baxcalibur,player-1,1|name:Flamethrower|target:Baxcalibur,player-2,1",
            "split|side:1",
            "damage|mon:Baxcalibur,player-2,1|health:81/175",
            "damage|mon:Baxcalibur,player-2,1|health:47/100",
            "move|mon:Baxcalibur,player-2,1|name:Recover|target:Baxcalibur,player-2,1",
            "split|side:1",
            "heal|mon:Baxcalibur,player-2,1|health:169/175",
            "heal|mon:Baxcalibur,player-2,1|health:97/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Baxcalibur,player-1,1|name:Flamethrower|target:Baxcalibur,player-2,1",
            "split|side:1",
            "damage|mon:Baxcalibur,player-2,1|health:135/175",
            "damage|mon:Baxcalibur,player-2,1|health:78/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
