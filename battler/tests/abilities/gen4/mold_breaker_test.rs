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

fn rampardos() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Rampardos",
                    "species": "Rampardos",
                    "ability": "Mold Breaker",
                    "moves": [
                        "Earthquake",
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

fn bastiodon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Bastiodon",
                    "species": "Bastiodon",
                    "ability": "Soundproof",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn gengar() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Gengar",
                    "species": "Gengar",
                    "ability": "Levitate",
                    "moves": [
                        "Gastro Acid"
                    ],
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn mold_breaker_suppresses_breakable_ability() {
    let mut battle = make_battle(0, rampardos().unwrap(), shedinja().unwrap()).unwrap();
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
            "ability|mon:Rampardos,player-1,1|ability:Mold Breaker",
            "turn|turn:1",
            ["time"],
            "move|mon:Rampardos,player-1,1|name:Earthquake",
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
fn mold_breaker_does_not_suppress_non_breakable_ability() {
    let mut battle = make_battle(0, rampardos().unwrap(), bastiodon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Rampardos,player-1,1|name:Hyper Voice|noanim",
            "immune|mon:Bastiodon,player-2,1|from:ability:Soundproof",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn mold_breaker_does_not_suppress_breakable_ability_with_ability_shield() {
    let mut gengar = gengar().unwrap();
    gengar.members[0].item = Some("Ability Shield".into());
    let mut battle = make_battle(0, rampardos().unwrap(), gengar).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Rampardos,player-1,1|name:Earthquake|noanim",
            "immune|mon:Gengar,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gastro_acid_suppresses_mold_breaker() {
    let mut battle = make_battle(0, rampardos().unwrap(), gengar().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-2,1|name:Gastro Acid|target:Rampardos,player-1,1",
            "abilityend|mon:Rampardos,player-1,1|ability:Mold Breaker|from:move:Gastro Acid|of:Gengar,player-2,1",
            "move|mon:Rampardos,player-1,1|name:Earthquake|noanim",
            "immune|mon:Gengar,player-2,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn gastro_acid_does_not_suppresses_mold_breaker_with_ability_shield() {
    let mut rampardos = rampardos().unwrap();
    rampardos.members[0].item = Some("Ability Shield".into());
    let mut battle = make_battle(0, rampardos, gengar().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Gengar,player-2,1|name:Gastro Acid|noanim",
            "block|mon:Rampardos,player-1,1|move:Gastro Acid|from:item:Ability Shield",
            "move|mon:Rampardos,player-1,1|name:Earthquake",
            "supereffective|mon:Gengar,player-2,1",
            "split|side:1",
            "damage|mon:Gengar,player-2,1|health:0",
            "damage|mon:Gengar,player-2,1|health:0",
            "faint|mon:Gengar,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
