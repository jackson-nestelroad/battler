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

fn togepi() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Togepi",
                    "species": "Togepi",
                    "ability": "No Ability",
                    "moves": [
                        "Encore",
                        "Growth",
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
fn encore_disables_all_moves_except_last_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, togepi().unwrap(), togepi().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0"),
        Err(err) => assert!(format!("{err:#}").contains("is disabled"))
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 2"),
        Err(err) => assert!(format!("{err:#}").contains("is disabled"))
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Togepi,player-1,1|name:Encore|noanim",
            "fail|mon:Togepi,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Togepi,player-2,1|name:Growth|target:Togepi,player-2,1",
            "boost|mon:Togepi,player-2,1|stat:atk|by:1",
            "boost|mon:Togepi,player-2,1|stat:spa|by:1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Togepi,player-1,1|name:Encore|target:Togepi,player-2,1",
            "start|mon:Togepi,player-2,1|move:Encore",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Togepi,player-2,1|name:Growth|target:Togepi,player-2,1",
            "boost|mon:Togepi,player-2,1|stat:atk|by:1",
            "boost|mon:Togepi,player-2,1|stat:spa|by:1",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Togepi,player-2,1|name:Growth|target:Togepi,player-2,1",
            "boost|mon:Togepi,player-2,1|stat:atk|by:1",
            "boost|mon:Togepi,player-2,1|stat:spa|by:1",
            "end|mon:Togepi,player-2,1|move:Encore",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Togepi,player-2,1|name:Tackle|target:Togepi,player-1,1",
            "split|side:0",
            "damage|mon:Togepi,player-1,1|health:78/95",
            "damage|mon:Togepi,player-1,1|health:83/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
