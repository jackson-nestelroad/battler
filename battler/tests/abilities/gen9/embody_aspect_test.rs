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
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Ogerpon",
                    "species": "Ogerpon",
                    "ability": "No Ability",
                    "moves": [
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "tera_type": "Grass"
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
        .with_terastallization(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn ogerpon_is_forced_to_have_grass_tera_type_and_transforms_on_terastallization() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Ogerpon,player-1,1|type:Grass",
            "split|side:0",
            ["specieschange", "player-1", "tera:Grass", "species:Ogerpon-Teal-Mask-Tera"],
            ["specieschange", "player-1", "tera:Grass", "species:Ogerpon-Teal-Mask-Tera"],
            "formechange|mon:Ogerpon,player-1,1|species:Ogerpon-Teal-Mask-Tera|from:species:Ogerpon",
            "boost|mon:Ogerpon,player-1,1|stat:spe|by:1|from:ability:Embody Aspect",
            "move|mon:Ogerpon,player-1,1|name:Splash|target:Ogerpon,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ogerpon_cornerstone_is_forced_to_have_rock_tera_type_and_transforms_on_terastallization() {
    let mut team_1 = team().unwrap();
    team_1.members[0].species = "Ogerpon-Cornerstone-Mask".to_owned();
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Ogerpon,player-1,1|type:Rock",
            "split|side:0",
            ["specieschange", "player-1", "tera:Rock", "species:Ogerpon-Cornerstone-Mask-Tera"],
            ["specieschange", "player-1", "tera:Rock", "species:Ogerpon-Cornerstone-Mask-Tera"],
            "formechange|mon:Ogerpon,player-1,1|species:Ogerpon-Cornerstone-Mask-Tera|from:species:Ogerpon-Cornerstone-Mask",
            "boost|mon:Ogerpon,player-1,1|stat:def|by:1|from:ability:Embody Aspect",
            "move|mon:Ogerpon,player-1,1|name:Splash|target:Ogerpon,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ogerpon_hearthflame_is_forced_to_have_fire_tera_type_and_transforms_on_terastallization() {
    let mut team_1 = team().unwrap();
    team_1.members[0].species = "Ogerpon-Hearthflame-Mask".to_owned();
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Ogerpon,player-1,1|type:Fire",
            "split|side:0",
            ["specieschange", "player-1", "tera:Fire", "species:Ogerpon-Hearthflame-Mask-Tera"],
            ["specieschange", "player-1", "tera:Fire", "species:Ogerpon-Hearthflame-Mask-Tera"],
            "formechange|mon:Ogerpon,player-1,1|species:Ogerpon-Hearthflame-Mask-Tera|from:species:Ogerpon-Hearthflame-Mask",
            "boost|mon:Ogerpon,player-1,1|stat:atk|by:1|from:ability:Embody Aspect",
            "move|mon:Ogerpon,player-1,1|name:Splash|target:Ogerpon,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn ogerpon_wellspring_is_forced_to_have_water_tera_type_and_transforms_on_terastallization() {
    let mut team_1 = team().unwrap();
    team_1.members[0].species = "Ogerpon-Wellspring-Mask".to_owned();
    let mut battle = make_battle(0, team_1, team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Ogerpon,player-1,1|type:Water",
            "split|side:0",
            ["specieschange", "player-1", "tera:Water", "species:Ogerpon-Wellspring-Mask-Tera"],
            ["specieschange", "player-1", "tera:Water", "species:Ogerpon-Wellspring-Mask-Tera"],
            "formechange|mon:Ogerpon,player-1,1|species:Ogerpon-Wellspring-Mask-Tera|from:species:Ogerpon-Wellspring-Mask",
            "boost|mon:Ogerpon,player-1,1|stat:spd|by:1|from:ability:Embody Aspect",
            "move|mon:Ogerpon,player-1,1|name:Splash|target:Ogerpon,player-1,1",
            "activate|move:Splash",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
