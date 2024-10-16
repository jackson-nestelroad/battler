use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn hariyama() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Hariyama",
                    "species": "Hariyama",
                    "ability": "No Ability",
                    "moves": [
                        "Smelling Salts",
                        "Thunder Wave"
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
) -> Result<PublicCoreBattle, Error> {
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
        .build(data)
}

#[test]
fn smelling_salts_doubles_power_against_paralysis() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, hariyama().unwrap(), hariyama().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Hariyama,player-1,1|name:Smelling Salts|target:Hariyama,player-2,1",
            "split|side:1",
            "damage|mon:Hariyama,player-2,1|health:145/204",
            "damage|mon:Hariyama,player-2,1|health:72/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Hariyama,player-1,1|name:Thunder Wave|target:Hariyama,player-2,1",
            "status|mon:Hariyama,player-2,1|status:Paralysis",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Hariyama,player-1,1|name:Smelling Salts|target:Hariyama,player-2,1",
            "split|side:1",
            "damage|mon:Hariyama,player-2,1|health:36/204",
            "damage|mon:Hariyama,player-2,1|health:18/100",
            "curestatus|mon:Hariyama,player-2,1|status:Paralysis",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
