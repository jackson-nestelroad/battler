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
                    "name": "Hydrapple",
                    "species": "Hydrapple",
                    "ability": "Sturdy",
                    "moves": [
                        "Fickle Beam"
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
fn fickle_beam_randomly_doubles_in_power() {
    let mut battle = make_battle(3, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hydrapple,player-1,1|name:Fickle Beam|target:Hydrapple,player-2,1|anim:All-Out",
            "activate|mon:Hydrapple,player-1,1|move:Fickle Beam",
            "supereffective|mon:Hydrapple,player-2,1",
            "activate|mon:Hydrapple,player-2,1|ability:Sturdy",
            "split|side:1",
            "damage|mon:Hydrapple,player-2,1|health:1/322",
            "damage|mon:Hydrapple,player-2,1|health:1/100",
            "move|mon:Hydrapple,player-2,1|name:Fickle Beam|target:Hydrapple,player-1,1",
            "supereffective|mon:Hydrapple,player-1,1",
            "split|side:0",
            "damage|mon:Hydrapple,player-1,1|health:44/322",
            "damage|mon:Hydrapple,player-1,1|health:14/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
