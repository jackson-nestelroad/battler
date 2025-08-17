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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn lopunny() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Lopunny",
                    "species": "Lopunny",
                    "ability": "No Ability",
                    "moves": [
                        "Healing Wish",
                        "Tackle",
                        "Spore"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn three_lopunny() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Lopunny",
                    "species": "Lopunny",
                    "ability": "No Ability",
                    "moves": [
                        "Healing Wish",
                        "Tackle",
                        "Spore"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Lopunny",
                    "species": "Lopunny",
                    "ability": "No Ability",
                    "moves": [
                        "Healing Wish",
                        "Tackle",
                        "Spore"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Lopunny",
                    "species": "Lopunny",
                    "ability": "No Ability",
                    "moves": [
                        "Healing Wish",
                        "Tackle",
                        "Spore"
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
fn healing_wish_fails_if_cannot_switch() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, lopunny().unwrap(), lopunny().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lopunny,player-1,1|name:Healing Wish|noanim",
            "fail|mon:Lopunny,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn healing_wish_faints_user_and_heals_slot() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 0, three_lopunny().unwrap(), three_lopunny().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Lopunny,player-1,1|name:Healing Wish|target:Lopunny,player-1,1",
            "faint|mon:Lopunny,player-1,1",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "health:125/125"],
            ["switch", "health:100/100"],
            "turn|turn:5",
            ["time"],
            "move|mon:Lopunny,player-2,1|name:Tackle|target:Lopunny,player-1,1",
            "split|side:0",
            "damage|mon:Lopunny,player-1,1|health:100/125",
            "damage|mon:Lopunny,player-1,1|health:80/100",
            "residual",
            "turn|turn:6",
            ["time"],
            "split|side:0",
            ["switch", "health:100/125", "status:Sleep"],
            ["switch", "health:80/100", "status:Sleep"],
            "split|side:0",
            "heal|mon:Lopunny,player-1,1|from:move:Healing Wish|health:125/125",
            "heal|mon:Lopunny,player-1,1|from:move:Healing Wish|health:100/100",
            "curestatus|mon:Lopunny,player-1,1|status:Sleep|from:move:Healing Wish",
            "residual",
            "turn|turn:7",
            ["time"],
            "split|side:0",
            ["switch", "health:100/125"],
            ["switch", "health:80/100"],
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 4, &expected_logs);
}
