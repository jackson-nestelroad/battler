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
                    "name": "Rapidash",
                    "species": "Rapidash-Galar",
                    "ability": "Pastel Veil",
                    "moves": [
                        "Toxic",
                        "Gastro Acid"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Darmanitan",
                    "species": "Darmanitan-Galar",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Darmanitan",
                    "species": "Darmanitan-Galar",
                    "ability": "No Ability",
                    "moves": [],
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
fn pastel_veil_prevents_poison_for_user_and_allies() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Rapidash,player-1,1|name:Toxic|noanim",
            "immune|mon:Rapidash,player-2,1|from:ability:Pastel Veil",
            "fail|mon:Rapidash,player-1,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Rapidash,player-1,1|name:Toxic|noanim",
            "block|mon:Darmanitan,player-2,2|move:Toxic|from:ability:Pastel Veil|of:Rapidash,player-2,1",
            "fail|mon:Rapidash,player-1,1",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn pastel_veil_heals_user_and_ally_poison_on_switch_in() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 2;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,1;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 0;pass"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "pass;switch 2"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Rapidash,player-1,1|name:Gastro Acid|target:Rapidash,player-2,1",
            "abilityend|mon:Rapidash,player-2,1|ability:Pastel Veil|from:move:Gastro Acid|of:Rapidash,player-1,1",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Rapidash,player-1,1|name:Toxic|target:Rapidash,player-2,1",
            "status|mon:Rapidash,player-2,1|status:Bad Poison",
            "split|side:1",
            "damage|mon:Rapidash,player-2,1|from:status:Bad Poison|health:225/240",
            "damage|mon:Rapidash,player-2,1|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:3",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Darmanitan"],
            ["switch", "player-2", "Darmanitan"],
            "move|mon:Rapidash,player-1,1|name:Toxic|target:Darmanitan,player-2,2",
            "status|mon:Darmanitan,player-2,2|status:Bad Poison",
            "split|side:1",
            "damage|mon:Darmanitan,player-2,2|from:status:Bad Poison|health:300/320",
            "damage|mon:Darmanitan,player-2,2|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Rapidash,player-1,1|name:Toxic|target:Darmanitan,player-2,1",
            "status|mon:Darmanitan,player-2,1|status:Bad Poison",
            "split|side:1",
            "damage|mon:Darmanitan,player-2,2|from:status:Bad Poison|health:260/320",
            "damage|mon:Darmanitan,player-2,2|from:status:Bad Poison|health:82/100",
            "split|side:1",
            "damage|mon:Darmanitan,player-2,1|from:status:Bad Poison|health:300/320",
            "damage|mon:Darmanitan,player-2,1|from:status:Bad Poison|health:94/100",
            "residual",
            "turn|turn:5",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Rapidash"],
            ["switch", "player-2", "Rapidash"],
            "activate|mon:Rapidash,player-2,1|ability:Pastel Veil",
            "curestatus|mon:Rapidash,player-2,1|status:Bad Poison|from:ability:Pastel Veil",
            "activate|mon:Rapidash,player-2,1|ability:Pastel Veil",
            "curestatus|mon:Darmanitan,player-2,2|status:Bad Poison|from:ability:Pastel Veil|of:Rapidash,player-2,1",
            "residual",
            "turn|turn:6",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Darmanitan"],
            ["switch", "player-2", "Darmanitan"],
            "activate|mon:Darmanitan,player-2,2|ability:Pastel Veil|of:Rapidash,player-2,1",
            "curestatus|mon:Darmanitan,player-2,2|status:Bad Poison|from:ability:Pastel Veil|of:Rapidash,player-2,1",
            "residual",
            "turn|turn:7"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
