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

fn team_1() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Venusaur",
                    "species": "Venusaur",
                    "ability": "Overgrow",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn team_2() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blastoise",
                    "species": "Blastoise",
                    "ability": "Torrent",
                    "moves": ["Tackle"],
                    "nature": "Modest",
                    "gender": "F",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle() -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(0)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_team("player-1", team_1()?)
        .with_team("player-2", team_2()?)
        .build(static_local_data_store())
}

#[test]
fn moves_can_be_used() {
    let mut battle = make_battle().unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    // Three turns of the Mons attacking each other with Tackle.
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    // Expected logs are simple.
    //
    // We don't check damage calculations since it does have a random factor.
    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:125/139",
            "damage|mon:Blastoise,player-2,1|health:90/100",
            "move|mon:Blastoise,player-2,1|name:Tackle|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:125/140",
            "damage|mon:Venusaur,player-1,1|health:90/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:113/139",
            "damage|mon:Blastoise,player-2,1|health:82/100",
            "move|mon:Blastoise,player-2,1|name:Tackle|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:110/140",
            "damage|mon:Venusaur,player-1,1|health:79/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Venusaur,player-1,1|name:Tackle|target:Blastoise,player-2,1",
            "split|side:1",
            "damage|mon:Blastoise,player-2,1|health:100/139",
            "damage|mon:Blastoise,player-2,1|health:72/100",
            "move|mon:Blastoise,player-2,1|name:Tackle|target:Venusaur,player-1,1",
            "split|side:0",
            "damage|mon:Venusaur,player-1,1|health:94/140",
            "damage|mon:Venusaur,player-1,1|health:68/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
