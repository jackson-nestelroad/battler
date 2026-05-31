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
                    "name": "Greedent",
                    "species": "Greedent",
                    "ability": "No Ability",
                    "item": "Sitrus Berry",
                    "moves": [
                        "Stuff Cheeks",
                        "Tackle"
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
fn stuff_cheeks_eats_berry_and_boosts_defense() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Greedent's Stuff Cheeks is disabled", "{err:?}")
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Greedent,player-1,1|name:Tackle|target:Greedent,player-2,1",
            "split|side:1",
            "damage|mon:Greedent,player-2,1|health:301/350",
            "damage|mon:Greedent,player-2,1|health:86/100",
            "move|mon:Greedent,player-2,1|name:Stuff Cheeks|target:Greedent,player-2,1",
            "boost|mon:Greedent,player-2,1|stat:def|by:2",
            "itemend|mon:Greedent,player-2,1|item:Sitrus Berry|eat",
            "split|side:1",
            "heal|mon:Greedent,player-2,1|from:item:Sitrus Berry|health:350/350",
            "heal|mon:Greedent,player-2,1|from:item:Sitrus Berry|health:100/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
