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

fn squirtle() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Squirtle",
                    "species": "Squirtle",
                    "ability": "No Ability",
                    "moves": [
                        "Explosion",
                        "Thunder Punch"
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
fn damp_prevents_self_destruct_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = squirtle().unwrap();
    player.members[0].ability = "Damp".to_owned();
    let mut battle = make_battle(&data, 0, player, squirtle().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Squirtle,player-2,1|name:Explosion|noanim",
            "cant|mon:Squirtle,player-2,1|from:ability:Damp|of:Squirtle,player-1,1",
            "fail|mon:Squirtle,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn damp_prevents_aftermath_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = squirtle().unwrap();
    player.members[0].ability = "Damp".to_owned();
    let mut opponent = squirtle().unwrap();
    opponent.members[0].ability = "Aftermath".to_owned();
    let mut battle = make_battle(&data, 0, player, opponent).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Squirtle,player-1,1|name:Thunder Punch|target:Squirtle,player-2,1",
            "supereffective|mon:Squirtle,player-2,1",
            "split|side:1",
            "damage|mon:Squirtle,player-2,1|health:0",
            "damage|mon:Squirtle,player-2,1|health:0",
            "faint|mon:Squirtle,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 3, &expected_logs);
}
