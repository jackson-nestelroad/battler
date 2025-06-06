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
                    "name": "Gardevoir",
                    "species": "Gardevoir",
                    "ability": "No Ability",
                    "moves": [
                        "Wish",
                        "Psychic"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Blaziken",
                    "species": "Blaziken",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
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
fn wish_heals_slot_on_next_turn() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gardevoir,player-1,1|name:Wish|target:Gardevoir,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Blaziken"],
            ["switch", "player-1", "Blaziken"],
            "move|mon:Gardevoir,player-2,1|name:Psychic|target:Blaziken,player-1,1",
            "supereffective|mon:Blaziken,player-1,1",
            "split|side:0",
            "damage|mon:Blaziken,player-1,1|health:166/270",
            "damage|mon:Blaziken,player-1,1|health:62/100",
            "split|side:0",
            "heal|mon:Blaziken,player-1,1|from:move:Wish|health:230/270",
            "heal|mon:Blaziken,player-1,1|from:move:Wish|health:86/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
