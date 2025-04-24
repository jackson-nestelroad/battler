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
                    "name": "Miltank",
                    "species": "Miltank",
                    "ability": "No Ability",
                    "moves": [
                        "Aromatherapy",
                        "Thunder Wave",
                        "Sleep Powder",
                        "Toxic"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Sceptile",
                    "species": "Sceptile",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Blaziken",
                    "species": "Blaziken",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Swampert",
                    "species": "Swampert",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Mightyena",
                    "species": "Mightyena",
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
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
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
fn aromatherapy_cures_all_statuses_on_side() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        team().unwrap(),
        team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Sceptile"],
            ["switch", "player-1", "Sceptile"],
            "move|mon:Miltank,player-2,1|name:Thunder Wave|target:Sceptile,player-1,1",
            "status|mon:Sceptile,player-1,1|status:Paralysis",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Blaziken"],
            ["switch", "player-1", "Blaziken"],
            "move|mon:Miltank,player-2,1|name:Sleep Powder|target:Blaziken,player-1,1",
            "status|mon:Blaziken,player-1,1|status:Sleep",
            "residual",
            "turn|turn:3",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Swampert"],
            ["switch", "player-1", "Swampert"],
            "move|mon:Miltank,player-2,1|name:Toxic|target:Swampert,player-1,1",
            "status|mon:Swampert,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Swampert,player-1,1|from:status:Bad Poison|health:150/160",
            "damage|mon:Swampert,player-1,1|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Miltank"],
            ["switch", "player-1", "Miltank"],
            "move|mon:Miltank,player-2,1|name:Toxic|target:Miltank,player-1,1",
            "status|mon:Miltank,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Miltank,player-1,1|from:status:Bad Poison|health:146/155",
            "damage|mon:Miltank,player-1,1|from:status:Bad Poison|health:95/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Aromatherapy",
            "activate|move:Aromatherapy|of:Miltank,player-1,1",
            "curestatus|mon:Miltank,player-1,1|status:Bad Poison",
            "curestatus|mon:Sceptile,player-1|status:Paralysis",
            "curestatus|mon:Blaziken,player-1|status:Sleep",
            "curestatus|mon:Swampert,player-1|status:Bad Poison",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn aromatherapy_activates_ally_sap_sipper() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

    let mut user = team().unwrap();
    user.members[0].ability = "Sap Sipper".to_owned();
    user.members[1].ability = "Sap Sipper".to_owned();
    user.members[2].ability = "Sap Sipper".to_owned();
    let mut battle = make_battle(&data, BattleType::Doubles, 0, user, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Miltank,player-2,1|name:Thunder Wave|target:Sceptile,player-1,2",
            "status|mon:Sceptile,player-1,2|status:Paralysis",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Miltank,player-1,1|name:Aromatherapy",
            "boost|mon:Sceptile,player-1,2|stat:atk|by:1",
            "activate|move:Aromatherapy|of:Miltank,player-1,1",
            "curestatus|mon:Sceptile,player-1,2|status:Paralysis",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
