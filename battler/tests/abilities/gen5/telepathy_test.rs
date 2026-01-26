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
                    "name": "Beheeyem",
                    "species": "Beheeyem",
                    "ability": "Telepathy",
                    "moves": [
                        "Surf"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Beheeyem",
                    "species": "Beheeyem",
                    "ability": "Telepathy",
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
fn telepathy_avoids_ally_attacks() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Beheeyem,player-1,1|name:Surf|spread:Beheeyem,player-2,1;Beheeyem,player-2,2",
            "activate|mon:Beheeyem,player-1,2|ability:Telepathy",
            "split|side:1",
            "damage|mon:Beheeyem,player-2,1|health:97/135",
            "damage|mon:Beheeyem,player-2,1|health:72/100",
            "split|side:1",
            "damage|mon:Beheeyem,player-2,2|health:99/135",
            "damage|mon:Beheeyem,player-2,2|health:74/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
