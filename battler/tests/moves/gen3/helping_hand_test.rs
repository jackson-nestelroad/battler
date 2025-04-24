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
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
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
                        "Helping Hand",
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
                        "Helping Hand",
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
) -> Result<PublicCoreBattle> {
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
fn helping_hand_boost_targets_power() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 1,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;move 1,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Plusle,player-1,1|name:Helping Hand|noanim",
            "fail|mon:Plusle,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Minun,player-1,2|name:Tackle|target:Plusle,player-2,1",
            "split|side:1",
            "damage|mon:Plusle,player-2,1|health:102/120",
            "damage|mon:Plusle,player-2,1|health:85/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Plusle,player-1,1|name:Helping Hand|target:Minun,player-1,2",
            "singleturn|mon:Minun,player-1,2|move:Helping Hand|of:Plusle,player-1,1",
            "move|mon:Minun,player-1,2|name:Tackle|target:Plusle,player-2,1",
            "split|side:1",
            "damage|mon:Plusle,player-2,1|health:77/120",
            "damage|mon:Plusle,player-2,1|health:65/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
