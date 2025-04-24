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
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "Static",
                    "moves": [
                        "Thunderbolt"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "No Ability",
                    "moves": [
                        "Follow Me"
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
fn lightning_rod_redirects_electric_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut opponent = team().unwrap();
    opponent.members[0].ability = "Lightning Rod".to_owned();
    let mut battle = make_battle(&data, 0, team().unwrap(), opponent).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "activate|mon:Pikachu,player-1,1|ability:Lightning Rod",
            "move|mon:Pikachu,player-1,1|name:Thunderbolt|target:Pikachu,player-2,1",
            "boost|mon:Pikachu,player-2,1|stat:spa|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn me_first_takes_priority_over_lightning_rod() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = team().unwrap();
    player.members[0].ability = "Lightning Rod".to_owned();
    let mut battle = make_battle(&data, 0, player, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;move 0"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,2;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Eevee,player-1,2|name:Follow Me|target:Eevee,player-1,2",
            "singleturn|mon:Eevee,player-1,2|move:Follow Me",
            "move|mon:Pikachu,player-2,1|name:Thunderbolt|target:Eevee,player-1,2",
            "split|side:0",
            "damage|mon:Eevee,player-1,2|health:67/115",
            "damage|mon:Eevee,player-1,2|health:59/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
