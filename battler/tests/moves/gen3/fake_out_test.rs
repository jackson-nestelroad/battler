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

fn skitty() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Skitty",
                    "species": "Skitty",
                    "ability": "No Ability",
                    "moves": [
                        "Fake Out"
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
fn fake_out_only_works_on_first_turn() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, skitty().unwrap(), skitty().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Skitty,player-1,1|name:Fake Out|target:Skitty,player-2,1",
            "split|side:1",
            "damage|mon:Skitty,player-2,1|health:83/110",
            "damage|mon:Skitty,player-2,1|health:76/100",
            "cant|mon:Skitty,player-2,1|from:Flinch",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Skitty,player-1,1|name:Fake Out|noanim",
            "fail|mon:Skitty,player-1,1",
            "move|mon:Skitty,player-2,1|name:Fake Out|noanim",
            "fail|mon:Skitty,player-2,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Skitty,player-1,1|name:Fake Out|noanim",
            "fail|mon:Skitty,player-1,1",
            "move|mon:Skitty,player-2,1|name:Fake Out|noanim",
            "fail|mon:Skitty,player-2,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
