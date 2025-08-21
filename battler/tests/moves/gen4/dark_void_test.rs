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
                    "name": "Darkrai",
                    "species": "Darkrai",
                    "ability": "No Ability",
                    "moves": [
                        "Dark Void"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "No Ability",
                    "moves": [
                        "Transform"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Smeargle",
                    "species": "Smeargle",
                    "ability": "No Ability",
                    "item": "Chesto Berry",
                    "moves": [
                        "Sketch"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Giratina",
                    "species": "Giratina",
                    "ability": "No Ability",
                    "moves": [
                        "Dark Void",
                        "Magic Coat"
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
fn dark_void_puts_all_adjacent_foes_to_sleep() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 953107372301, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darkrai,player-1,1|name:Dark Void|spread:Darkrai,player-2,1;Ditto,player-2,2",
            "status|mon:Darkrai,player-2,1|status:Sleep",
            "status|mon:Ditto,player-2,2|status:Sleep",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dark_void_usable_when_transformed() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 953107372301, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ditto,player-2,2|name:Transform|target:Darkrai,player-1,1",
            "transform|mon:Ditto,player-2,2|into:Darkrai,player-1,1|species:Darkrai",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-2,2|name:Dark Void|spread:Darkrai,player-1,1;Ditto,player-1,2",
            "status|mon:Darkrai,player-1,1|status:Sleep",
            "status|mon:Ditto,player-1,2|status:Sleep",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dark_void_cannot_be_sketched() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 953107372301, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darkrai,player-1,1|name:Dark Void|spread:Darkrai,player-2,1;Ditto,player-2,2",
            "status|mon:Darkrai,player-2,1|status:Sleep",
            "status|mon:Ditto,player-2,2|status:Sleep",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:1",
            ["switch", "Smeargle"],
            ["switch", "Smeargle"],
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Smeargle,player-2,2|name:Sketch|noanim",
            "fail|mon:Smeargle,player-2,2",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dark_void_cannot_be_used_by_non_darkrai() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 953107372301, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 3;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "Giratina"],
            ["switch", "Giratina"],
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Giratina,player-1,1|name:Dark Void|noanim",
            "fail|mon:Giratina,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn dark_void_can_be_reflected() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 953107372301, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 3;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "Giratina"],
            ["switch", "Giratina"],
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Giratina,player-2,1|name:Magic Coat|target:Giratina,player-2,1",
            "singleturn|mon:Giratina,player-2,1|move:Magic Coat",
            "move|mon:Darkrai,player-1,1|name:Dark Void|noanim",
            "activate|mon:Giratina,player-2,1|move:Magic Coat",
            "move|mon:Giratina,player-2,1|name:Dark Void|from:move:Magic Coat|noanim",
            "status|mon:Darkrai,player-1,1|status:Sleep",
            "status|mon:Ditto,player-1,2|status:Sleep",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
