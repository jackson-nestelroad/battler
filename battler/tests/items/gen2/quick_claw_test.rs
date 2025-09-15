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
    get_controlled_rng_for_battle,
    static_local_data_store,
};

fn totodile() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Totodile",
                    "species": "Totodile",
                    "ability": "Torrent",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Quick Claw"
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn quick_claw_allows_holder_to_move_first() {
    let mut battle = make_battle(0, totodile().unwrap(), totodile().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 1), (2, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "activate|mon:Totodile,player-2,1|item:Quick Claw",
            ["time"],
            "move|mon:Totodile,player-2,1|name:Tackle|target:Totodile,player-1,1",
            "split|side:0",
            "damage|mon:Totodile,player-1,1|health:91/110",
            "damage|mon:Totodile,player-1,1|health:83/100",
            "move|mon:Totodile,player-1,1|name:Tackle|target:Totodile,player-2,1",
            "split|side:1",
            "damage|mon:Totodile,player-2,1|health:92/110",
            "damage|mon:Totodile,player-2,1|health:84/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
