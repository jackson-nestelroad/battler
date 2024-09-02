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
    assert_error_message,
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn swampert() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Swampert",
                    "species": "Swampert",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle",
                        "Hyper Voice"
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
) -> Result<PublicCoreBattle, Error> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn choice_band_boosts_attack_and_locks_choice() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = swampert().unwrap();
    team.members[0].item = Some("Choice Band".to_owned());
    let mut battle = make_battle(&data, 0, team, swampert().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_error_message(
        battle.set_player_choice("player-1", "move 1"),
        "cannot move: Swampert's Hyper Voice is disabled",
    );

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swampert,player-1,1|name:Tackle|target:Swampert,player-2,1",
            "split|side:1",
            "damage|mon:Swampert,player-2,1|health:127/160",
            "damage|mon:Swampert,player-2,1|health:80/100",
            "move|mon:Swampert,player-2,1|name:Tackle|target:Swampert,player-1,1",
            "split|side:0",
            "damage|mon:Swampert,player-1,1|health:137/160",
            "damage|mon:Swampert,player-1,1|health:86/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Swampert,player-1,1|name:Tackle|target:Swampert,player-2,1",
            "split|side:1",
            "damage|mon:Swampert,player-2,1|health:94/160",
            "damage|mon:Swampert,player-2,1|health:59/100",
            "move|mon:Swampert,player-2,1|name:Tackle|target:Swampert,player-1,1",
            "split|side:0",
            "damage|mon:Swampert,player-1,1|health:114/160",
            "damage|mon:Swampert,player-1,1|health:72/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn choice_scarf_boosts_speed_and_locks_choice() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = swampert().unwrap();
    team.members[0].item = Some("Choice Scarf".to_owned());
    let mut battle = make_battle(&data, 0, team, swampert().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_error_message(
        battle.set_player_choice("player-1", "move 1"),
        "cannot move: Swampert's Hyper Voice is disabled",
    );

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swampert,player-1,1|name:Tackle|target:Swampert,player-2,1",
            "split|side:1",
            "damage|mon:Swampert,player-2,1|health:137/160",
            "damage|mon:Swampert,player-2,1|health:86/100",
            "move|mon:Swampert,player-2,1|name:Tackle|target:Swampert,player-1,1",
            "split|side:0",
            "damage|mon:Swampert,player-1,1|health:137/160",
            "damage|mon:Swampert,player-1,1|health:86/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Swampert,player-1,1|name:Tackle|target:Swampert,player-2,1",
            "split|side:1",
            "damage|mon:Swampert,player-2,1|health:114/160",
            "damage|mon:Swampert,player-2,1|health:72/100",
            "move|mon:Swampert,player-2,1|name:Tackle|target:Swampert,player-1,1",
            "split|side:0",
            "damage|mon:Swampert,player-1,1|health:114/160",
            "damage|mon:Swampert,player-1,1|health:72/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn choice_specs_boosts_special_attack_and_locks_choice() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = swampert().unwrap();
    team.members[0].item = Some("Choice Specs".to_owned());
    let mut battle = make_battle(&data, 0, team, swampert().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_error_message(
        battle.set_player_choice("player-1", "move 0"),
        "cannot move: Swampert's Tackle is disabled",
    );

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swampert,player-1,1|name:Hyper Voice",
            "split|side:1",
            "damage|mon:Swampert,player-2,1|health:102/160",
            "damage|mon:Swampert,player-2,1|health:64/100",
            "move|mon:Swampert,player-2,1|name:Hyper Voice",
            "split|side:0",
            "damage|mon:Swampert,player-1,1|health:121/160",
            "damage|mon:Swampert,player-1,1|health:76/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Swampert,player-1,1|name:Hyper Voice",
            "split|side:1",
            "damage|mon:Swampert,player-2,1|health:44/160",
            "damage|mon:Swampert,player-2,1|health:28/100",
            "move|mon:Swampert,player-2,1|name:Hyper Voice",
            "split|side:0",
            "damage|mon:Swampert,player-1,1|health:82/160",
            "damage|mon:Swampert,player-1,1|health:52/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
