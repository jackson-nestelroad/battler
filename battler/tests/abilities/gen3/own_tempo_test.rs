use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    get_controlled_rng_for_battle,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData, Error> {
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

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle, Error> {
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
        .build(data)
}

#[test]
fn own_tempo_prevents_confusion() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

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
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(1, 99)]);

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "switch 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            ["switch", "player-1", "Trapinch"],
            "move|mon:Spinda,player-2,1|name:Confuse Ray|target:Trapinch,player-1,1",
            "start|mon:Trapinch,player-1,1|condition:Confusion",
            "residual",
            "turn|turn:2",
            ["time"],
            "activate|mon:Trapinch,player-1,1|condition:Confusion",
            "move|mon:Trapinch,player-1,1|name:Baton Pass|target:Trapinch,player-1,1",
            ["time"],
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
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "switch 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
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
