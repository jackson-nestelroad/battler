use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    Error,
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

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Fly",
                        "Flamethrower"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                },
                {
                    "name": "Alakazam",
                    "species": "Alakazam",
                    "ability": "No Ability",
                    "moves": [
                        "Counter",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
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
fn counter_doubles_damage_of_last_physical_hit_on_user() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 1000, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;move 0"),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0;move 1,2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Fly|noanim",
            "prepare|mon:Charizard,player-2,1|move:Fly",
            "move|mon:Alakazam,player-1,2|name:Counter|noanim",
            "fail|mon:Alakazam,player-1,2",
            "move|mon:Alakazam,player-2,2|name:Counter|noanim",
            "fail|mon:Alakazam,player-2,2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Alakazam,player-2,2|name:Tackle|target:Alakazam,player-1,2",
            "split|side:0",
            "damage|mon:Alakazam,player-1,2|health:96/115",
            "damage|mon:Alakazam,player-1,2|health:84/100",
            "move|mon:Charizard,player-2,1|name:Fly|target:Alakazam,player-1,2",
            "split|side:0",
            "damage|mon:Alakazam,player-1,2|health:3/115",
            "damage|mon:Alakazam,player-1,2|health:3/100",
            "move|mon:Alakazam,player-1,2|name:Counter|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:0",
            "damage|mon:Charizard,player-2,1|health:0",
            "faint|mon:Charizard,player-2,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn counter_does_not_counter_special_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 1000, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;move 0"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Flamethrower|target:Alakazam,player-1,2",
            "split|side:0",
            "damage|mon:Alakazam,player-1,2|health:48/115",
            "damage|mon:Alakazam,player-1,2|health:42/100",
            "move|mon:Alakazam,player-1,2|name:Counter|noanim",
            "fail|mon:Alakazam,player-1,2",
            "move|mon:Alakazam,player-2,2|name:Counter|noanim",
            "fail|mon:Alakazam,player-2,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
