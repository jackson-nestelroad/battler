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

fn shuppet() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Shuppet",
                    "species": "Shuppet",
                    "ability": "No Ability",
                    "moves": [
                        "Snatch",
                        "Flamethrower",
                        "Amnesia"
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
fn snatch_steals_beneficial_status_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, shuppet().unwrap(), shuppet().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Shuppet,player-1,1|name:Snatch|target:Shuppet,player-1,1",
            "move|mon:Shuppet,player-2,1|name:Flamethrower|target:Shuppet,player-1,1",
            "split|side:0",
            "damage|mon:Shuppet,player-1,1|health:35/104",
            "damage|mon:Shuppet,player-1,1|health:34/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Shuppet,player-1,1|name:Snatch|target:Shuppet,player-1,1",
            "move|mon:Shuppet,player-2,1|name:Amnesia|target:Shuppet,player-2,1",
            "activate|mon:Shuppet,player-1,1|move:Snatch",
            "move|mon:Shuppet,player-1,1|name:Amnesia|target:Shuppet,player-1,1|from:move:Snatch",
            "boost|mon:Shuppet,player-1,1|stat:spd|by:2",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
