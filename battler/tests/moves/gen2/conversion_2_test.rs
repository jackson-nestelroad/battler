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

fn porygon2() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Porygon2",
                    "species": "Porygon2",
                    "ability": "No Ability",
                    "moves": [
                        "Conversion 2",
                        "Tackle",
                        "Water Gun"
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
fn conversion_2_changes_type_based_on_targets_last_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, porygon2().unwrap(), porygon2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Porygon2,player-1,1|name:Conversion 2|noanim",
            "fail|mon:Porygon2,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Porygon2,player-2,1|name:Tackle|target:Porygon2,player-1,1",
            "split|side:0",
            "damage|mon:Porygon2,player-1,1|health:121/145",
            "damage|mon:Porygon2,player-1,1|health:84/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Porygon2,player-1,1|name:Conversion 2|target:Porygon2,player-2,1",
            "typechange|mon:Porygon2,player-1,1|types:Rock",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Porygon2,player-1,1|name:Conversion 2|target:Porygon2,player-2,1",
            "typechange|mon:Porygon2,player-1,1|types:Ghost",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Porygon2,player-2,1|name:Water Gun|target:Porygon2,player-1,1",
            "split|side:0",
            "damage|mon:Porygon2,player-1,1|health:102/145",
            "damage|mon:Porygon2,player-1,1|health:71/100",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Porygon2,player-1,1|name:Conversion 2|target:Porygon2,player-2,1",
            "typechange|mon:Porygon2,player-1,1|types:Dragon",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
