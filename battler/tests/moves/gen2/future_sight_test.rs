use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    error::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn xatu() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Xatu",
                    "species": "Xatu",
                    "ability": "No Ability",
                    "moves": [
                        "Future Sight"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Typhlosion",
                    "species": "Typhlosion",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn ampharos_machamp() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ampharos",
                    "species": "Ampharos",
                    "ability": "No Ability",
                    "moves": [
                        "Thunderbolt"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Machamp",
                    "species": "Machamp",
                    "ability": "No Ability",
                    "moves": [],
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
fn future_sight_attacks_slot_three_turns_later() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, xatu().unwrap(), ampharos_machamp().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Xatu,player-1,1|name:Future Sight|noanim",
            "start|move:Future Sight|of:Xatu,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Xatu,player-1,1|name:Future Sight|noanim",
            "fail|mon:Xatu,player-1,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "end|move:Future Sight|mon:Machamp,player-2,2|of:Xatu,player-1,1",
            "animatemove|mon:Xatu,player-1,1|name:Future Sight|target:Machamp,player-2,2",
            "supereffective|mon:Machamp,player-2,2",
            "split|side:1",
            "damage|mon:Machamp,player-2,2|health:0",
            "damage|mon:Machamp,player-2,2|health:0",
            "faint|mon:Machamp,player-2,2",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn future_sight_attacks_even_if_user_faints() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, xatu().unwrap(), ampharos_machamp().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Xatu,player-1,1|name:Future Sight|noanim",
            "start|move:Future Sight|of:Xatu,player-1,1",
            "move|mon:Ampharos,player-2,1|name:Thunderbolt|target:Xatu,player-1,1",
            "supereffective|mon:Xatu,player-1,1",
            "split|side:0",
            "damage|mon:Xatu,player-1,1|health:0",
            "damage|mon:Xatu,player-1,1|health:0",
            "faint|mon:Xatu,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "residual",
            "turn|turn:3",
            ["time"],
            "end|move:Future Sight|mon:Machamp,player-2,2|of:Xatu,player-1",
            "animatemove|mon:Xatu,player-1|name:Future Sight|target:Machamp,player-2,2",
            "supereffective|mon:Machamp,player-2,2",
            "split|side:1",
            "damage|mon:Machamp,player-2,2|health:0",
            "damage|mon:Machamp,player-2,2|health:0",
            "faint|mon:Machamp,player-2,2",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
