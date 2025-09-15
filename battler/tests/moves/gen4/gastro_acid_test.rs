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
    assert_logs_since_start_eq,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn swalot() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Swalot",
                    "species": "Swalot",
                    "ability": "No Ability",
                    "moves": [
                        "Gastro Acid",
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
                    "moves": [
                        "Baton Pass"
                    ],
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

fn psyduck_castform() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Psyduck",
                    "species": "Psyduck",
                    "ability": "No Ability",
                    "moves": [
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

fn make_battle(
    battle_type: BattleType,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn gastro_acid_suppresses_ability() {
    let mut battle = make_battle(
        BattleType::Singles,
        0,
        swalot().unwrap(),
        shedinja().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swalot,player-1,1|name:Gastro Acid|target:Shedinja,player-2,1",
            "abilityend|mon:Shedinja,player-2,1|ability:Wonder Guard|from:move:Gastro Acid|of:Swalot,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Swalot,player-1,1|name:Sludge|target:Shedinja,player-2,1",
            "resisted|mon:Shedinja,player-2,1",
            "split|side:1",
            "damage|mon:Shedinja,player-2,1|health:0",
            "damage|mon:Shedinja,player-2,1|health:0",
            "faint|mon:Shedinja,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gastro_acid_fails_on_unsuppressible_ability() {
    let mut battle = make_battle(
        BattleType::Singles,
        0,
        swalot().unwrap(),
        komala().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swalot,player-1,1|name:Gastro Acid|noanim",
            "fail|mon:Swalot,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gastro_acid_cannot_be_passed_to_unsuppressible_ability() {
    let mut team = shedinja().unwrap();
    team.members.extend(komala().unwrap().members);
    let mut battle = make_battle(BattleType::Singles, 0, swalot().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swalot,player-1,1|name:Gastro Acid|target:Shedinja,player-2,1",
            "abilityend|mon:Shedinja,player-2,1|ability:Wonder Guard|from:move:Gastro Acid|of:Swalot,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Shedinja,player-2,1|name:Baton Pass|target:Shedinja,player-2,1",
            "switchout|mon:Shedinja,player-2,1",
            ["time"],
            "split|side:1",
            "switch|player:player-2|position:1|name:Komala|health:125/125|species:Komala|level:50|gender:U",
            "switch|player:player-2|position:1|name:Komala|health:100/100|species:Komala|level:50|gender:U",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Komala,player-2,1|name:Snore|target:Swalot,player-1,1",
            "split|side:0",
            "damage|mon:Swalot,player-1,1|health:129/160",
            "damage|mon:Swalot,player-1,1|health:81/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gastro_acid_triggers_ability_end() {
    let mut team = psyduck_castform().unwrap();
    team.members[0].ability = "Cloud Nine".to_owned();
    let mut battle = make_battle(
        BattleType::Doubles,
        0,
        psyduck_castform().unwrap(),
        team,
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

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
            "ability|mon:Psyduck,player-2,1|ability:Cloud Nine",
            "turn|turn:1",
            ["time"],
            "move|mon:Psyduck,player-1,1|name:Gastro Acid|target:Psyduck,player-2,1",
            "abilityend|mon:Psyduck,player-2,1|ability:Cloud Nine|from:move:Gastro Acid|of:Psyduck,player-1,1",
            "formechange|mon:Castform,player-1,2|species:Castform-Rainy|from:ability:Forecast",
            "formechange|mon:Castform,player-2,2|species:Castform-Rainy|from:ability:Forecast",
            "weather|weather:Rain|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_start_eq(&battle, &expected_logs);
}
