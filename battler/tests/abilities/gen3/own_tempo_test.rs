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
    get_controlled_rng_for_battle,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Spinda",
                    "species": "Spinda",
                    "ability": "Own Tempo",
                    "moves": [
                        "Confuse Ray"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Trapinch",
                    "species": "Trapinch",
                    "ability": "No Ability",
                    "moves": [
                        "Baton Pass"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Poochyena",
                    "species": "Poochyena",
                    "ability": "Intimidate",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
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
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn own_tempo_prevents_confusion() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Spinda,player-2,1|name:Confuse Ray|target:Spinda,player-1,1",
            "immune|mon:Spinda,player-1,1|from:ability:Own Tempo",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn own_tempo_heals_confusion_on_baton_pass() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 99)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch", "player-1", "Trapinch"],
            ["switch", "player-1", "Trapinch"],
            "move|mon:Spinda,player-2,1|name:Confuse Ray|target:Trapinch,player-1,1",
            "start|mon:Trapinch,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:2",
            "continue",
            "activate|mon:Trapinch,player-1,1|condition:Confusion",
            "move|mon:Trapinch,player-1,1|name:Baton Pass|target:Trapinch,player-1,1",
            "switchout|mon:Trapinch,player-1,1",
            "continue",
            "split|side:0",
            ["switch", "player-1", "Spinda"],
            ["switch", "player-1", "Spinda"],
            "activate|mon:Spinda,player-1,1|ability:Own Tempo",
            "end|mon:Spinda,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn own_tempo_resists_intimidate() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:1",
            ["switch", "player-2", "Poochyena"],
            ["switch", "player-2", "Poochyena"],
            "activate|mon:Poochyena,player-2,1|ability:Intimidate",
            "fail|mon:Spinda,player-1,1|what:unboost|boosts:atk|from:ability:Own Tempo",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
