use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Plusle",
                    "species": "Plusle",
                    "ability": "No Ability",
                    "moves": [
                        "Follow Me",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Minun",
                    "species": "Minun",
                    "ability": "No Ability",
                    "moves": [
                        "Follow Me",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
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
        .build(data)
}

#[test]
fn follow_me_redirects_foe_moves_to_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0;move 1,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;move 1,2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Plusle,player-1,1|name:Follow Me|target:Plusle,player-1,1",
            "singleturn|mon:Plusle,player-1,1|move:Follow Me",
            "move|mon:Minun,player-1,2|name:Tackle|target:Minun,player-2,2",
            "split|side:1",
            "damage|mon:Minun,player-2,2|health:105/120",
            "damage|mon:Minun,player-2,2|health:88/100",
            "move|mon:Plusle,player-2,1|name:Tackle|target:Plusle,player-1,1",
            "split|side:0",
            "damage|mon:Plusle,player-1,1|health:100/120",
            "damage|mon:Plusle,player-1,1|health:84/100",
            "move|mon:Minun,player-2,2|name:Tackle|target:Plusle,player-1,1",
            "split|side:0",
            "damage|mon:Plusle,player-1,1|health:84/120",
            "damage|mon:Plusle,player-1,1|health:70/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
