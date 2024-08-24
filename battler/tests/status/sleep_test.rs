use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
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
    LogMatch,
    TestBattleBuilder,
};

fn charizard() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Charizard",
                    "species": "Charizard",
                    "ability": "No Ability",
                    "moves": [
                        "Sleep Powder",
                        "Tackle",
                        "Snore"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
                    "ball": "Normal",
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
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn sleep_prevents_movement_until_waking_up() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        910624796386061,
        charizard().unwrap(),
        charizard().unwrap(),
    )
    .unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Sleep Powder|target:Charizard,player-1,1",
            "status|mon:Charizard,player-1,1|status:Sleep|from:move:Sleep Powder",
            "cant|mon:Charizard,player-1,1|reason:Sleep",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Tackle|target:Charizard,player-1,1",
            "split|side:0",
            "damage|mon:Charizard,player-1,1|health:118/138",
            "damage|mon:Charizard,player-1,1|health:86/100",
            "cant|mon:Charizard,player-1,1|reason:Sleep",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Tackle|target:Charizard,player-1,1",
            "split|side:0",
            "damage|mon:Charizard,player-1,1|health:98/138",
            "damage|mon:Charizard,player-1,1|health:72/100",
            "cant|mon:Charizard,player-1,1|reason:Sleep",
            "residual",
            "turn|turn:4",
            ["time"],
            "move|mon:Charizard,player-2,1|name:Tackle|target:Charizard,player-1,1",
            "split|side:0",
            "damage|mon:Charizard,player-1,1|health:78/138",
            "damage|mon:Charizard,player-1,1|health:57/100",
            "curestatus|mon:Charizard,player-1,1|status:Sleep",
            "move|mon:Charizard,player-1,1|name:Tackle|target:Charizard,player-2,1",
            "split|side:1",
            "damage|mon:Charizard,player-2,1|health:118/138",
            "damage|mon:Charizard,player-2,1|health:86/100",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn sleep_usable_moves_can_only_be_used_while_asleep() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        632700081244845,
        charizard().unwrap(),
        charizard().unwrap(),
    )
    .unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Charizard,player-2,1|name:Snore|noanim",
            "fail|mon:Charizard,player-2,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Charizard,player-1,1|name:Sleep Powder|target:Charizard,player-2,1",
            "status|mon:Charizard,player-2,1|status:Sleep|from:move:Sleep Powder",
            "residual",
            "turn|turn:3",
            ["time"],
            "cant|mon:Charizard,player-2,1|reason:Sleep",
            "move|mon:Charizard,player-2,1|name:Snore|target:Charizard,player-1,1",
            "split|side:0",
            "damage|mon:Charizard,player-1,1|health:109/138",
            "damage|mon:Charizard,player-1,1|health:79/100",
            "residual",
            "turn|turn:4",
            ["time"],
            "curestatus|mon:Charizard,player-2,1|status:Sleep",
            "move|mon:Charizard,player-2,1|name:Snore|noanim",
            "fail|mon:Charizard,player-2,1",
            "residual",
            "turn|turn:5"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
