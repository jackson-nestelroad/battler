use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn gigalith() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gigalith",
                    "species": "Gigalith",
                    "ability": "No Ability",
                    "moves": [
                        "Earthquake",
                        "Tackle",
                        "Substitute"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Air Balloon"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn air_balloon_lifts_mon_until_popped() {
    let mut battle = make_battle(0, gigalith().unwrap(), gigalith().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gigalith,player-1,1|name:Earthquake|noanim",
            "immune|mon:Gigalith,player-2,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Gigalith,player-1,1|name:Tackle|target:Gigalith,player-2,1",
            "resisted|mon:Gigalith,player-2,1",
            "split|side:1",
            "damage|mon:Gigalith,player-2,1|health:136/145",
            "damage|mon:Gigalith,player-2,1|health:94/100",
            "itemend|mon:Gigalith,player-2,1|item:Air Balloon",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Gigalith,player-1,1|name:Earthquake",
            "supereffective|mon:Gigalith,player-2,1",
            "split|side:1",
            "damage|mon:Gigalith,player-2,1|health:52/145",
            "damage|mon:Gigalith,player-2,1|health:36/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn air_balloon_pops_behind_substitute() {
    let mut battle = make_battle(0, gigalith().unwrap(), gigalith().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gigalith,player-1,1|name:Substitute|target:Gigalith,player-1,1",
            "start|mon:Gigalith,player-1,1|move:Substitute",
            "split|side:0",
            "damage|mon:Gigalith,player-1,1|health:109/145",
            "damage|mon:Gigalith,player-1,1|health:76/100",
            "move|mon:Gigalith,player-2,1|name:Tackle|target:Gigalith,player-1,1",
            "resisted|mon:Gigalith,player-1,1",
            "activate|mon:Gigalith,player-1,1|move:Substitute|damage",
            "itemend|mon:Gigalith,player-1,1|item:Air Balloon",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
