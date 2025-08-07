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

fn dark_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Tyranitar",
                    "species": "Tyranitar",
                    "ability": "No Ability",
                    "moves": [
                        "Pursuit"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Umbreon",
                    "species": "Umbreon",
                    "ability": "No Ability",
                    "moves": [
                        "Pursuit"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn psychic_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Espeon",
                    "species": "Espeon",
                    "ability": "No Ability",
                    "moves": [
                        "U-turn"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Lugia",
                    "species": "Lugia",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Celebi",
                    "species": "Celebi",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Xatu",
                    "species": "Xatu",
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
fn pursuit_works_without_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, dark_team().unwrap(), psychic_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Tyranitar,player-1,1|name:Pursuit|target:Espeon,player-2,1",
            "supereffective|mon:Espeon,player-2,1",
            "split|side:1",
            "damage|mon:Espeon,player-2,1|health:15/125",
            "damage|mon:Espeon,player-2,1|health:12/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn pursuit_runs_before_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 100, dark_team().unwrap(), psychic_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "activate|mon:Espeon,player-2,1|move:Pursuit",
            "move|mon:Tyranitar,player-1,1|name:Pursuit|target:Espeon,player-2,1",
            "supereffective|mon:Espeon,player-2,1",
            "split|side:1",
            "damage|mon:Espeon,player-2,1|health:0",
            "damage|mon:Espeon,player-2,1|health:0",
            "faint|mon:Espeon,player-2,1",
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Celebi"],
            ["switch", "player-2", "Celebi"],
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn multiple_pursuits_at_the_same_time() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 100, dark_team().unwrap(), psychic_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;switch 3"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "activate|mon:Espeon,player-2,1|move:Pursuit",
            "move|mon:Umbreon,player-1,2|name:Pursuit|target:Espeon,player-2,1",
            "supereffective|mon:Espeon,player-2,1",
            "split|side:1",
            "damage|mon:Espeon,player-2,1|health:27/125",
            "damage|mon:Espeon,player-2,1|health:22/100",
            "move|mon:Tyranitar,player-1,1|name:Pursuit|target:Espeon,player-2,1",
            "supereffective|mon:Espeon,player-2,1",
            "split|side:1",
            "damage|mon:Espeon,player-2,1|health:0",
            "damage|mon:Espeon,player-2,1|health:0",
            "faint|mon:Espeon,player-2,1",
            "split|side:1",
            ["switch", "player-2", "Xatu"],
            ["switch", "player-2", "Xatu"],
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Celebi"],
            ["switch", "player-2", "Celebi"],
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn pursuit_activates_on_user_switch_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, dark_team().unwrap(), psychic_team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Espeon,player-2,1|name:U-turn|target:Tyranitar,player-1,1",
            "split|side:0",
            "damage|mon:Tyranitar,player-1,1|health:141/160",
            "damage|mon:Tyranitar,player-1,1|health:89/100",
            "activate|mon:Espeon,player-2,1|move:Pursuit",
            "move|mon:Tyranitar,player-1,1|name:Pursuit|target:Espeon,player-2,1",
            "supereffective|mon:Espeon,player-2,1",
            "split|side:1",
            "damage|mon:Espeon,player-2,1|health:0",
            "damage|mon:Espeon,player-2,1|health:0",
            "faint|mon:Espeon,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
