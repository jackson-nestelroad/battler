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
                    "name": "Swampert",
                    "species": "Swampert",
                    "ability": "Torrent",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Slaking",
                    "species": "Slaking",
                    "ability": "Truant",
                    "moves": [
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
fn truant_causes_user_to_slack_off() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Slaking"],
            ["switch", "player-2", "Slaking"],
            "move|mon:Slaking,player-1,1|name:Tackle|target:Slaking,player-2,1",
            "split|side:1",
            "damage|mon:Slaking,player-2,1|health:168/210",
            "damage|mon:Slaking,player-2,1|health:80/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "cant|mon:Slaking,player-1,1|from:ability:Truant",
            "move|mon:Slaking,player-2,1|name:Tackle|target:Slaking,player-1,1",
            "split|side:0",
            "damage|mon:Slaking,player-1,1|health:171/210",
            "damage|mon:Slaking,player-1,1|health:82/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Slaking,player-1,1|name:Tackle|target:Slaking,player-2,1",
            "split|side:1",
            "damage|mon:Slaking,player-2,1|health:132/210",
            "damage|mon:Slaking,player-2,1|health:63/100",
            "cant|mon:Slaking,player-2,1|from:ability:Truant",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
