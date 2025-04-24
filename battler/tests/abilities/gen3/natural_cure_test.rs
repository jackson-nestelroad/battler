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

fn team_1() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Roselia",
                    "species": "Roselia",
                    "ability": "Natural Cure",
                    "moves": [
                        "Flamethrower"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Swablu",
                    "species": "Swablu",
                    "ability": "Natural Cure",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn team_2() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Budew",
                    "species": "Budew",
                    "ability": "Natural Cure",
                    "moves": [
                        "Thunder Wave",
                        "Will-O-Wisp"
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
fn natural_cure_heals_status_on_switch_out() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Budew,player-2,1|name:Thunder Wave|target:Roselia,player-1,1",
            "status|mon:Roselia,player-1,1|status:Paralysis",
            "residual",
            "turn|turn:2",
            ["time"],
            "curestatus|mon:Roselia,player-1,1|status:Paralysis|from:ability:Natural Cure",
            "split|side:0",
            ["switch", "player-1", "Swablu"],
            ["switch", "player-1", "Swablu"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn natural_cure_heals_status_on_battle_end() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team_1().unwrap(), team_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Budew,player-2,1|name:Will-O-Wisp|target:Roselia,player-1,1",
            "status|mon:Roselia,player-1,1|status:Burn",
            "split|side:0",
            "damage|mon:Roselia,player-1,1|from:status:Burn|health:104/110",
            "damage|mon:Roselia,player-1,1|from:status:Burn|health:95/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Roselia,player-1,1|name:Flamethrower|target:Budew,player-2,1",
            "supereffective|mon:Budew,player-2,1",
            "split|side:1",
            "damage|mon:Budew,player-2,1|health:0",
            "damage|mon:Budew,player-2,1|health:0",
            "faint|mon:Budew,player-2,1",
            "curestatus|mon:Roselia,player-1,1|status:Burn|from:ability:Natural Cure",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
