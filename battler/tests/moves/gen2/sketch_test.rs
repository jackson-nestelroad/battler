use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn smeargle_1() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Smeargle",
                    "species": "Smeargle",
                    "ability": "No Ability",
                    "moves": [
                        "Sketch",
                        "Water Gun"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn smeargle_2() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Smeargle",
                    "species": "Smeargle",
                    "ability": "No Ability",
                    "moves": [
                        "Sketch",
                        "Flamethrower"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn sketch_fails_for_no_last_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, smeargle_1().unwrap(), smeargle_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Smeargle,player-1,1|name:Sketch|noanim",
            "fail|mon:Smeargle,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sketch_copies_targets_last_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, smeargle_1().unwrap(), smeargle_2().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Smeargle,player-1,1|name:Water Gun|target:Smeargle,player-2,1",
            "split|side:1",
            "damage|mon:Smeargle,player-2,1|health:106/115",
            "damage|mon:Smeargle,player-2,1|health:93/100",
            "move|mon:Smeargle,player-2,1|name:Flamethrower|target:Smeargle,player-1,1",
            "split|side:0",
            "damage|mon:Smeargle,player-1,1|health:97/115",
            "damage|mon:Smeargle,player-1,1|health:85/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Smeargle,player-1,1|name:Sketch|target:Smeargle,player-2,1",
            "activate|move:Sketch|mon:Smeargle,player-1,1|newmove:Flamethrower",
            "move|mon:Smeargle,player-2,1|name:Sketch|noanim",
            "fail|mon:Smeargle,player-2,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Smeargle,player-1,1|name:Flamethrower|target:Smeargle,player-2,1",
            "split|side:1",
            "damage|mon:Smeargle,player-2,1|health:86/115",
            "damage|mon:Smeargle,player-2,1|health:75/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
