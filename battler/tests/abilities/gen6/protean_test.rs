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
                    "name": "Greninja",
                    "species": "Greninja",
                    "ability": "Protean",
                    "item": "Greninjite",
                    "moves": [
                        "Flamethrower",
                        "Water Gun",
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
        .with_mega_evolution(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn protean_changes_type_to_match_move_once() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Greninja,player-1,1|name:Flamethrower|target:Greninja,player-2,1",
            "typechange|mon:Greninja,player-1,1|types:Fire|from:ability:Protean",
            "resisted|mon:Greninja,player-2,1",
            "split|side:1",
            "damage|mon:Greninja,player-2,1|health:175/254",
            "damage|mon:Greninja,player-2,1|health:69/100",
            "move|mon:Greninja,player-2,1|name:Water Gun|target:Greninja,player-1,1",
            "typechange|mon:Greninja,player-2,1|types:Water|from:ability:Protean",
            "supereffective|mon:Greninja,player-1,1",
            "split|side:0",
            "damage|mon:Greninja,player-1,1|health:120/254",
            "damage|mon:Greninja,player-1,1|health:48/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Greninja,player-1,1|name:Water Gun|target:Greninja,player-2,1",
            "resisted|mon:Greninja,player-2,1",
            "split|side:1",
            "damage|mon:Greninja,player-2,1|health:151/254",
            "damage|mon:Greninja,player-2,1|health:60/100",
            "move|mon:Greninja,player-2,1|name:Flamethrower|target:Greninja,player-1,1",
            "resisted|mon:Greninja,player-1,1",
            "split|side:0",
            "damage|mon:Greninja,player-1,1|health:68/254",
            "damage|mon:Greninja,player-1,1|health:27/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn protean_resets_after_mega_evolution() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Greninja,player-1,1|name:Flamethrower|target:Greninja,player-2,1",
            "typechange|mon:Greninja,player-1,1|types:Fire|from:ability:Protean",
            "resisted|mon:Greninja,player-2,1",
            "split|side:1",
            "damage|mon:Greninja,player-2,1|health:175/254",
            "damage|mon:Greninja,player-2,1|health:69/100",
            "residual",
            "turn|turn:2",
            "continue",
            "split|side:0",
            ["specieschange", "player-1", "Greninja-Mega"],
            ["specieschange", "player-1", "Greninja-Mega"],
            "mega|mon:Greninja,player-1,1|species:Greninja-Mega|from:item:Greninjite",
            "move|mon:Greninja,player-1,1|name:Tackle|target:Greninja,player-2,1",
            "typechange|mon:Greninja,player-1,1|types:Normal|from:ability:Protean",
            "split|side:1",
            "damage|mon:Greninja,player-2,1|health:90/254",
            "damage|mon:Greninja,player-2,1|health:36/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
