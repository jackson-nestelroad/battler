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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Swampert",
                    "species": "Swampert",
                    "ability": "No Ability",
                    "moves": [
                        "Substitute"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Mightyena",
                    "species": "Mightyena",
                    "ability": "Intimidate",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Manectric",
                    "species": "Manectric",
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
fn intimidate_lowers_adjacent_foes_attack_on_appearance() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Swampert"],
            ["switch", "player-1", "Swampert"],
            "split|side:0",
            ["switch", "player-1", "Mightyena"],
            ["switch", "player-1", "Mightyena"],
            "split|side:1",
            ["switch", "player-2", "Swampert"],
            ["switch", "player-2", "Swampert"],
            "split|side:1",
            ["switch", "player-2", "Mightyena"],
            ["switch", "player-2", "Mightyena"],
            "activate|mon:Mightyena,player-2,2|ability:Intimidate",
            "unboost|mon:Swampert,player-1,1|stat:atk|by:1",
            "unboost|mon:Mightyena,player-1,2|stat:atk|by:1",
            "activate|mon:Mightyena,player-1,2|ability:Intimidate",
            "unboost|mon:Swampert,player-2,1|stat:atk|by:1",
            "unboost|mon:Mightyena,player-2,2|stat:atk|by:1",
            "turn|turn:1",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Manectric"],
            ["switch", "player-1", "Manectric"],
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Mightyena"],
            ["switch", "player-1", "Mightyena"],
            "activate|mon:Mightyena,player-1,1|ability:Intimidate",
            "unboost|mon:Swampert,player-2,1|stat:atk|by:1",
            "unboost|mon:Mightyena,player-2,2|stat:atk|by:1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn substitute_resists_intimidate() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;switch 2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Manectric"],
            ["switch", "player-1", "Manectric"],
            "move|mon:Swampert,player-2,1|name:Substitute|target:Swampert,player-2,1",
            "start|mon:Swampert,player-2,1|move:Substitute",
            "split|side:1",
            "damage|mon:Swampert,player-2,1|health:120/160",
            "damage|mon:Swampert,player-2,1|health:75/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Mightyena"],
            ["switch", "player-1", "Mightyena"],
            "activate|mon:Mightyena,player-1,1|ability:Intimidate",
            "fail|mon:Swampert,player-2,1|what:unboost|from:move:Substitute",
            "unboost|mon:Mightyena,player-2,2|stat:atk|by:1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
