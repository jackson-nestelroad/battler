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
                    "name": "Pikachu",
                    "species": "Pikachu",
                    "ability": "No Ability",
                    "moves": [
                        "Tackle"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Magikarp",
                    "species": "Magikarp",
                    "ability": "No Ability",
                    "moves": [
                        "Splash"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Eevee",
                    "species": "Eevee",
                    "ability": "No Ability",
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

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Doubles)
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
fn outside_effects_trigger_immediately_at_start_of_turn() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.push_outside_effect(
            serde_json::from_str(
                r#"{
                    "name": "Toxic Fumes",
                    "target": "field",
                    "program": [
                        "foreach $mon in func_call(all_active_mons):",
                        [
                            "if func_call(chance: 1 2):",
                            [
                                "set_status: $mon tox"
                            ]
                        ]
                    ]
                }"#,
            )
            .unwrap(),
        ),
        Ok(())
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 2;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 0"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "status|mon:Pikachu,player-1,1|status:Bad Poison|from:Toxic Fumes",
            "status|mon:Magikarp,player-1,2|status:Bad Poison|from:Toxic Fumes",
            "status|mon:Magikarp,player-2,2|status:Bad Poison|from:Toxic Fumes",
            "split|side:0",
            ["switch", "player-1", "Eevee"],
            ["switch", "player-1", "Eevee"],
            "move|mon:Pikachu,player-2,1|name:Tackle|target:Eevee,player-1,1",
            "split|side:0",
            "damage|mon:Eevee,player-1,1|health:96/115",
            "damage|mon:Eevee,player-1,1|health:84/100",
            "move|mon:Magikarp,player-1,2|name:Splash|target:Magikarp,player-1,2",
            "activate|move:Splash",
            "move|mon:Magikarp,player-2,2|name:Splash|target:Magikarp,player-2,2",
            "activate|move:Splash",
            "split|side:0",
            "damage|mon:Magikarp,player-1,2|from:status:Bad Poison|health:75/80",
            "damage|mon:Magikarp,player-1,2|from:status:Bad Poison|health:94/100",
            "split|side:1",
            "damage|mon:Magikarp,player-2,2|from:status:Bad Poison|health:75/80",
            "damage|mon:Magikarp,player-2,2|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn outside_effect_targets_individual_mon() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.push_outside_effect(
            serde_json::from_str(
                r#"{
                    "name": "Faint",
                    "target": {
                        "mon": {
                            "player": "player-2",
                            "position": 0
                        }
                    },
                    "program": [
                        "faint: $target"
                    ]
                }"#,
            )
            .unwrap(),
        ),
        Ok(())
    );

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;move 0"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 0,1;move 0"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "faint|mon:Pikachu,player-2,1",
            "move|mon:Pikachu,player-1,1|name:Tackle|target:Magikarp,player-2,2",
            "split|side:1",
            "damage|mon:Magikarp,player-2,2|health:62/80",
            "damage|mon:Magikarp,player-2,2|health:78/100",
            "move|mon:Magikarp,player-1,2|name:Splash|target:Magikarp,player-1,2",
            "activate|move:Splash",
            "move|mon:Magikarp,player-2,2|name:Splash|target:Magikarp,player-2,2",
            "activate|move:Splash",
            "residual"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
