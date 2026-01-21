use anyhow::Result;
use battler::{
    battle::{
        BattleType,
        CoreBattleEngineRandomizeBaseDamage,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    teams::TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};
use serde_json;

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Torracat",
                    "species": "Torracat",
                    "ability": "No Ability",
                    "moves": ["Burn Up"],
                    "nature": "Hardy",
                    "level": 50,
                    "tera_type": "Fire"
                },
                {
                    "name": "Turtonator",
                    "species": "Turtonator",
                    "ability": "No Ability",
                    "moves": ["Burn Up"],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(0)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_terastallization(true)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn burn_up_pure_fire_type_becomes_typeless() {
    let mut battle = make_battle(BattleType::Singles, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Torracat,player-1,1|name:Burn Up|target:Torracat,player-2,1",
            "resisted|mon:Torracat,player-2,1",
            "split|side:1",
            "damage|mon:Torracat,player-2,1|health:58/125",
            "damage|mon:Torracat,player-2,1|health:47/100",
            "typechange|mon:Torracat,player-1,1|types:None",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Torracat,player-1,1|name:Burn Up|noanim",
            "fail|mon:Torracat,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn burn_up_dual_type_loses_fire_type() {
    let mut battle = make_battle(BattleType::Singles, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            "switch|player:player-1|position:1|name:Turtonator|health:120/120|species:Turtonator|level:50|gender:U",
            "switch|player:player-1|position:1|name:Turtonator|health:100/100|species:Turtonator|level:50|gender:U",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Turtonator,player-1,1|name:Burn Up|target:Torracat,player-2,1",
            "resisted|mon:Torracat,player-2,1",
            "split|side:1",
            "damage|mon:Torracat,player-2,1|health:50/125",
            "damage|mon:Torracat,player-2,1|health:40/100",
            "typechange|mon:Turtonator,player-1,1|types:Dragon",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Turtonator,player-1,1|name:Burn Up|noanim",
            "fail|mon:Turtonator,player-1,1",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn burn_up_terastallized_remains_fire_type() {
    let mut battle = make_battle(BattleType::Singles, team(), team()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1,tera"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "tera|mon:Torracat,player-1,1|type:Fire",
            "move|mon:Torracat,player-1,1|name:Burn Up|target:Torracat,player-2,1",
            "resisted|mon:Torracat,player-2,1",
            "split|side:1",
            "damage|mon:Torracat,player-2,1|health:35/125",
            "damage|mon:Torracat,player-2,1|health:28/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Torracat,player-1,1|name:Burn Up|target:Torracat,player-2,1",
            "resisted|mon:Torracat,player-2,1",
            "split|side:1",
            "damage|mon:Torracat,player-2,1|health:0",
            "damage|mon:Torracat,player-2,1|health:0",
            "faint|mon:Torracat,player-2,1",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}