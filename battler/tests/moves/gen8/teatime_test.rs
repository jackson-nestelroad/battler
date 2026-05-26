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
                    "name": "Polteageist",
                    "species": "Polteageist",
                    "ability": "No Ability",
                    "item": "Sitrus Berry",
                    "moves": [
                        "Teatime",
                        "Stomping Tantrum"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Polteageist",
                    "species": "Polteageist",
                    "ability": "No Ability",
                    "item": "Liechi Berry",
                    "moves": [
                        "Teatime",
                        "Stomping Tantrum"
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
fn teatime_forces_all_mons_to_eat_berry() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Polteageist,player-1,1|name:Teatime|spread:Polteageist,player-1,1;Polteageist,player-1,2;Polteageist,player-2,1;Polteageist,player-2,2",
            "fieldactivate|move:Teatime",
            "itemend|mon:Polteageist,player-1,1|item:Sitrus Berry|eat",
            "itemend|mon:Polteageist,player-1,2|item:Liechi Berry|eat",
            "boost|mon:Polteageist,player-1,2|stat:atk|by:1|from:item:Liechi Berry|of:Polteageist,player-1,1",
            "itemend|mon:Polteageist,player-2,1|item:Sitrus Berry|eat",
            "itemend|mon:Polteageist,player-2,2|item:Liechi Berry|eat",
            "boost|mon:Polteageist,player-2,2|stat:atk|by:1|from:item:Liechi Berry|of:Polteageist,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn teatime_fails_if_no_mons_have_berry() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;move 1,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Polteageist,player-1,1|name:Teatime|spread",
            "fail|mon:Polteageist,player-1,1",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Polteageist,player-1,1|name:Stomping Tantrum|target:Polteageist,player-2,1",
            "split|side:1",
            "damage|mon:Polteageist,player-2,1|health:167/230",
            "damage|mon:Polteageist,player-2,1|health:73/100",
            "move|mon:Polteageist,player-1,2|name:Stomping Tantrum|target:Polteageist,player-2,2",
            "split|side:1",
            "damage|mon:Polteageist,player-2,2|health:144/230",
            "damage|mon:Polteageist,player-2,2|health:63/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
