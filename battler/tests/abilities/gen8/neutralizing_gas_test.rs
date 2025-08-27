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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
};

fn weezing() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Weezing",
                    "species": "Weezing",
                    "ability": "Neutralizing Gas",
                    "moves": [
                        "Sludge"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn shedinja() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Shedinja",
                    "species": "Shedinja",
                    "ability": "Wonder Guard",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn komala() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Komala",
                    "species": "Komala",
                    "ability": "Comatose",
                    "moves": [
                        "Snore"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn koffing_mightyena() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Koffing",
                    "species": "Koffing",
                    "ability": "Neutralizing Gas",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Mightyena",
                    "species": "Mightyena",
                    "ability": "Intimidate",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn koffing_ditto() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Koffing",
                    "species": "Koffing",
                    "ability": "Neutralizing Gas",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 1
                },
                {
                    "name": "Ditto",
                    "species": "Ditto",
                    "ability": "No Ability",
                    "moves": [
                        "Transform",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn psyduck_castform() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Psyduck",
                    "species": "Psyduck",
                    "ability": "Cloud Nine",
                    "moves": [
                        "Tackle",
                        "Gastro Acid"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Castform",
                    "species": "Castform",
                    "ability": "Forecast",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn ninetales_weezing() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ninetales",
                    "species": "Ninetales",
                    "ability": "Flash Fire",
                    "moves": [
                        "Flamethrower"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Weezing",
                    "species": "Weezing",
                    "ability": "Neutralizing Gas",
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
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_weather(if battle_type == BattleType::Doubles {
            Some("rainweather".to_owned())
        } else {
            None
        })
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn neutralizing_gas_suppresses_ability() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        weezing().unwrap(),
        shedinja().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "ability|mon:Weezing,player-1,1|ability:Neutralizing Gas",
            "turn|turn:1",
            ["time"],
            "move|mon:Weezing,player-1,1|name:Sludge|target:Shedinja,player-2,1",
            "resisted|mon:Shedinja,player-2,1",
            "split|side:1",
            "damage|mon:Shedinja,player-2,1|health:0",
            "damage|mon:Shedinja,player-2,1|health:0",
            "faint|mon:Shedinja,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn neutralizing_gas_ignores_unsuppressible_ability() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        weezing().unwrap(),
        komala().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Komala,player-2,1|name:Snore|target:Weezing,player-1,1",
            "split|side:0",
            "damage|mon:Weezing,player-1,1|health:89/125",
            "damage|mon:Weezing,player-1,1|health:72/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn neutralizing_gas_ends_ability_on_appearance() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        ninetales_weezing().unwrap(),
        ninetales_weezing().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ninetales,player-1,1|name:Flamethrower|target:Ninetales,player-2,1",
            "start|mon:Ninetales,player-2,1|ability:Flash Fire",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            "switch|player:player-1|position:1|name:Weezing|health:125/125|species:Weezing|level:50|gender:U",
            "switch|player:player-1|position:1|name:Weezing|health:100/100|species:Weezing|level:50|gender:U",
            "ability|mon:Weezing,player-1,1|ability:Neutralizing Gas",
            "end|mon:Ninetales,player-2,1|ability:Flash Fire|silent",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn neutralizing_gas_does_not_end_ability_on_appearance_with_ability_shield() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = ninetales_weezing().unwrap();
    team.members[0].item = Some("Ability Shield".to_owned());
    let mut battle = make_battle(
        &data,
        BattleType::Singles,
        0,
        ninetales_weezing().unwrap(),
        team,
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Ninetales,player-1,1|name:Flamethrower|target:Ninetales,player-2,1",
            "start|mon:Ninetales,player-2,1|ability:Flash Fire",
            "residual",
            "turn|turn:2",
            ["time"],
            "split|side:0",
            "switch|player:player-1|position:1|name:Weezing|health:125/125|species:Weezing|level:50|gender:U",
            "switch|player:player-1|position:1|name:Weezing|health:100/100|species:Weezing|level:50|gender:U",
            "ability|mon:Weezing,player-1,1|ability:Neutralizing Gas",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn neutralizing_gas_restarts_abilities_on_exit() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Doubles,
        0,
        koffing_mightyena().unwrap(),
        psyduck_castform().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Rain|from:Start",
            "ability|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "turn|turn:1",
            ["time"],
            "move|mon:Psyduck,player-2,1|name:Tackle|target:Koffing,player-1,1",
            "split|side:0",
            "damage|mon:Koffing,player-1,1|health:0",
            "damage|mon:Koffing,player-1,1|health:0",
            "faint|mon:Koffing,player-1,1",
            "end|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "activate|mon:Mightyena,player-1,2|ability:Intimidate",
            "unboost|mon:Psyduck,player-2,1|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,2",
            "unboost|mon:Castform,player-2,2|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn neutralizing_gas_restarts_abilities_on_end() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Doubles,
        0,
        koffing_mightyena().unwrap(),
        psyduck_castform().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Rain|from:Start",
            "ability|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "turn|turn:1",
            ["time"],
            "move|mon:Psyduck,player-2,1|name:Gastro Acid|target:Koffing,player-1,1",
            "abilityend|mon:Koffing,player-1,1|ability:Neutralizing Gas|from:move:Gastro Acid|of:Psyduck,player-2,1",
            "end|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "activate|mon:Mightyena,player-1,2|ability:Intimidate",
            "unboost|mon:Psyduck,player-2,1|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,2",
            "unboost|mon:Castform,player-2,2|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,2",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn neutralizing_gas_does_not_suppress_with_ability_shield() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = psyduck_castform().unwrap();
    team.members[1].item = Some("Ability Shield".to_owned());
    let mut battle = make_battle(
        &data,
        BattleType::Doubles,
        0,
        koffing_mightyena().unwrap(),
        team,
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;pass"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Rain|from:Start",
            "ability|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "formechange|mon:Castform,player-2,2|species:Castform-Rainy|from:ability:Forecast",
            "turn|turn:1",
            ["time"],
            "move|mon:Psyduck,player-2,1|name:Tackle|target:Koffing,player-1,1",
            "split|side:0",
            "damage|mon:Koffing,player-1,1|health:0",
            "damage|mon:Koffing,player-1,1|health:0",
            "faint|mon:Koffing,player-1,1",
            "end|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "activate|mon:Mightyena,player-1,2|ability:Intimidate",
            "unboost|mon:Psyduck,player-2,1|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,2",
            "unboost|mon:Castform,player-2,2|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,2",
            "formechange|mon:Castform,player-2,2|species:Castform|from:ability:Forecast",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn neutralizing_gas_does_not_end_when_another_mon_has_ability() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Doubles,
        0,
        koffing_mightyena().unwrap(),
        koffing_ditto().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 1,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Rain|from:Start",
            "ability|mon:Koffing,player-2,1|ability:Neutralizing Gas",
            "ability|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "turn|turn:1",
            ["time"],
            "move|mon:Mightyena,player-1,2|name:Tackle|target:Koffing,player-2,1",
            "split|side:1",
            "damage|mon:Koffing,player-2,1|health:0",
            "damage|mon:Koffing,player-2,1|health:0",
            "faint|mon:Koffing,player-2,1",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-2,2|name:Tackle|target:Koffing,player-1,1",
            "split|side:0",
            "damage|mon:Koffing,player-1,1|health:0",
            "damage|mon:Koffing,player-1,1|health:0",
            "faint|mon:Koffing,player-1,1",
            "end|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "activate|mon:Mightyena,player-1,2|ability:Intimidate",
            "unboost|mon:Ditto,player-2,2|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,2",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}

#[test]
fn neutralizing_gas_does_not_activate_when_transformed() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        BattleType::Doubles,
        0,
        koffing_mightyena().unwrap(),
        koffing_ditto().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "pass;move 0,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;move 0,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:0",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "split|side:1",
            ["switch"],
            ["switch"],
            "weather|weather:Rain|from:Start",
            "ability|mon:Koffing,player-2,1|ability:Neutralizing Gas",
            "ability|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "turn|turn:1",
            ["time"],
            "move|mon:Mightyena,player-1,2|name:Tackle|target:Koffing,player-2,1",
            "split|side:1",
            "damage|mon:Koffing,player-2,1|health:0",
            "damage|mon:Koffing,player-2,1|health:0",
            "faint|mon:Koffing,player-2,1",
            "move|mon:Ditto,player-2,2|name:Transform|target:Koffing,player-1,1",
            "transform|mon:Ditto,player-2,2|into:Koffing,player-1,1|species:Koffing",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Ditto,player-2,2|name:Tackle|target:Koffing,player-1,1",
            "split|side:0",
            "damage|mon:Koffing,player-1,1|health:0",
            "damage|mon:Koffing,player-1,1|health:0",
            "faint|mon:Koffing,player-1,1",
            "end|mon:Koffing,player-1,1|ability:Neutralizing Gas",
            "activate|mon:Mightyena,player-1,2|ability:Intimidate",
            "unboost|mon:Ditto,player-2,2|stat:atk|by:1|from:ability:Intimidate|of:Mightyena,player-1,2",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
