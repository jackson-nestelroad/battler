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
                    "name": "Shiinotic",
                    "species": "Shiinotic",
                    "ability": "No Ability",
                    "moves": [
                        "Strength Sap",
                        "Mist"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "persistent_battle_data": {
                        "hp": 1
                    }
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
fn strength_sap_drops_target_attack_and_heals_user() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Shiinotic,player-1,1|name:Strength Sap|target:Shiinotic,player-2,1",
            "unboost|mon:Shiinotic,player-2,1|stat:atk|by:1",
            "split|side:0",
            "heal|mon:Shiinotic,player-1,1|health:96/230",
            "heal|mon:Shiinotic,player-1,1|health:42/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Shiinotic,player-1,1|name:Strength Sap|target:Shiinotic,player-2,1",
            "unboost|mon:Shiinotic,player-2,1|stat:atk|by:1",
            "split|side:0",
            "heal|mon:Shiinotic,player-1,1|health:159/230",
            "heal|mon:Shiinotic,player-1,1|health:70/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn strength_sap_fails_if_target_attack_cannot_be_lowered() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.push_outside_effect(
            serde_json::from_str(
                r#"{
                    "name": "Drop Attack",
                    "target": {
                        "mon": {
                            "player": "player-2",
                            "position": 0
                        }
                    },
                    "program": [
                        "boost: $target 'atk:-6'"
                    ]
                }"#,
            )
            .unwrap(),
        ),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "unboost|mon:Shiinotic,player-2,1|stat:atk|by:6|from:Drop Attack",
            "move|mon:Shiinotic,player-1,1|name:Strength Sap|noanim",
            "fail|mon:Shiinotic,player-1,1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn strength_sap_does_not_fail_if_only_heal_succeeds() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Shiinotic,player-2,1|name:Mist",
            "sidestart|side:1|move:Mist",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Shiinotic,player-1,1|name:Strength Sap|target:Shiinotic,player-2,1",
            "activate|mon:Shiinotic,player-2,1|move:Mist",
            "split|side:0",
            "heal|mon:Shiinotic,player-1,1|health:96/230",
            "heal|mon:Shiinotic,player-1,1|health:42/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn strength_sap_does_not_fail_if_only_boost_succeeds() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.push_outside_effect(
            serde_json::from_str(
                r#"{
                    "name": "Heal",
                    "target": {
                        "mon": {
                            "player": "player-1",
                            "position": 0
                        }
                    },
                    "program": [
                        "heal: $target $target.max_hp"
                    ]
                }"#,
            )
            .unwrap(),
        ),
        Ok(())
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            "heal|mon:Shiinotic,player-1,1|from:Heal|health:230/230",
            "heal|mon:Shiinotic,player-1,1|from:Heal|health:100/100",
            "move|mon:Shiinotic,player-1,1|name:Strength Sap|target:Shiinotic,player-2,1",
            "unboost|mon:Shiinotic,player-2,1|stat:atk|by:1",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
