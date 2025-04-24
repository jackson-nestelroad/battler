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

fn level_100_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Wobbuffet",
                    "species": "Wobbuffet",
                    "ability": "No Ability",
                    "moves": [
                        "Destiny Bond",
                        "Dark Pulse",
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Qwilfish",
                    "species": "Qwilfish",
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

fn level_50_team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Wobbuffet",
                    "species": "Wobbuffet",
                    "ability": "No Ability",
                    "moves": [
                        "Destiny Bond",
                        "Dark Pulse",
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Qwilfish",
                    "species": "Qwilfish",
                    "ability": "No Ability",
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
fn destiny_bond_faints_attacking_mon() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        level_50_team().unwrap(),
        level_100_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wobbuffet,player-2,1|name:Dark Pulse|target:Wobbuffet,player-1,1",
            "supereffective|mon:Wobbuffet,player-1,1",
            "split|side:0",
            "damage|mon:Wobbuffet,player-1,1|health:102/250",
            "damage|mon:Wobbuffet,player-1,1|health:41/100",
            "move|mon:Wobbuffet,player-1,1|name:Destiny Bond|target:Wobbuffet,player-1,1",
            "singlemove|mon:Wobbuffet,player-1,1|move:Destiny Bond",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Wobbuffet,player-2,1|name:Dark Pulse|target:Wobbuffet,player-1,1",
            "supereffective|mon:Wobbuffet,player-1,1",
            "split|side:0",
            "damage|mon:Wobbuffet,player-1,1|health:0",
            "damage|mon:Wobbuffet,player-1,1|health:0",
            "faint|mon:Wobbuffet,player-1,1",
            "activate|mon:Wobbuffet,player-1,1|move:Destiny Bond",
            "faint|mon:Wobbuffet,player-2,1",
            "residual",
            ["time"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn destiny_bond_resets_when_using_another_move() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        level_50_team().unwrap(),
        level_100_team().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Wobbuffet,player-2,1|name:Dark Pulse|target:Wobbuffet,player-1,1",
            "supereffective|mon:Wobbuffet,player-1,1",
            "split|side:0",
            "damage|mon:Wobbuffet,player-1,1|health:102/250",
            "damage|mon:Wobbuffet,player-1,1|health:41/100",
            "move|mon:Wobbuffet,player-1,1|name:Destiny Bond|target:Wobbuffet,player-1,1",
            "singlemove|mon:Wobbuffet,player-1,1|move:Destiny Bond",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Wobbuffet,player-1,1|name:Splash|target:Wobbuffet,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Wobbuffet,player-2,1|name:Dark Pulse|target:Wobbuffet,player-1,1",
            "supereffective|mon:Wobbuffet,player-1,1",
            "split|side:0",
            "damage|mon:Wobbuffet,player-1,1|health:0",
            "damage|mon:Wobbuffet,player-1,1|health:0",
            "faint|mon:Wobbuffet,player-1,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
