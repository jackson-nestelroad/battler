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

fn scizor() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Scizor",
                    "species": "Scizor",
                    "ability": "No Ability",
                    "moves": [
                        "Fury Cutter"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn blissey() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Blissey",
                    "species": "Blissey",
                    "ability": "No Ability",
                    "moves": [
                        "Rest"
                    ],
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
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn fury_cutter_doubles_power_for_consecutive_hits() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 12345, scizor().unwrap(), blissey().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Scizor,player-1,1|name:Fury Cutter|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:475/620",
            "damage|mon:Blissey,player-2,1|health:77/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Scizor,player-1,1|name:Fury Cutter|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:187/620",
            "damage|mon:Blissey,player-2,1|health:31/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Scizor,player-1,1|name:Fury Cutter|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:0",
            "damage|mon:Blissey,player-2,1|health:0",
            "faint|mon:Blissey,player-2,1",
            "residual",
            ["time"],
            "split|side:1",
            ["switch", "player-2", "Blissey"],
            ["switch", "player-2", "Blissey"],
            "turn|turn:4",
            ["time"],
            "move|mon:Scizor,player-1,1|name:Fury Cutter|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:47/620",
            "damage|mon:Blissey,player-2,1|health:8/100",
            "residual",
            "turn|turn:5",
            ["time"],
            "move|mon:Blissey,player-2,1|name:Rest|target:Blissey,player-2,1",
            "status|mon:Blissey,player-2,1|status:Sleep",
            "split|side:1",
            "heal|mon:Blissey,player-2,1|health:620/620",
            "heal|mon:Blissey,player-2,1|health:100/100",
            "residual",
            "turn|turn:6",
            ["time"],
            "move|mon:Scizor,player-1,1|name:Fury Cutter|target:Blissey,player-2,1",
            "split|side:1",
            "damage|mon:Blissey,player-2,1|health:475/620",
            "damage|mon:Blissey,player-2,1|health:77/100",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
