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
                    "name": "Cresselia",
                    "species": "Cresselia",
                    "ability": "No Ability",
                    "moves": [
                        "Lunar Blessing",
                        "Toxic"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "persistent_battle_data": {
                        "hp": 50
                    }
                },
                {
                    "name": "Darkrai",
                    "species": "Darkrai",
                    "ability": "No Ability",
                    "moves": [
                        "Toxic"
                    ],
                    "nature": "Hardy",
                    "level": 100,
                    "persistent_battle_data": {
                        "hp": 50
                    }
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
fn lunar_blessing_restores_hp_and_cures_status() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 0,2"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Darkrai,player-2,2|name:Toxic|target:Darkrai,player-1,2",
            "status|mon:Darkrai,player-1,2|status:Bad Poison",
            "move|mon:Cresselia,player-1,1|name:Lunar Blessing|spread:Cresselia,player-1,1;Darkrai,player-1,2",
            "split|side:0",
            "heal|mon:Cresselia,player-1,1|health:138/350",
            "heal|mon:Cresselia,player-1,1|health:40/100",
            "split|side:0",
            "heal|mon:Darkrai,player-1,2|health:138/250",
            "heal|mon:Darkrai,player-1,2|health:56/100",
            "curestatus|mon:Darkrai,player-1,2|status:Bad Poison",
            "move|mon:Cresselia,player-2,1|name:Toxic|target:Cresselia,player-1,1",
            "status|mon:Cresselia,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Cresselia,player-1,1|from:status:Bad Poison|health:117/350",
            "damage|mon:Cresselia,player-1,1|from:status:Bad Poison|health:34/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cresselia,player-1,1|name:Lunar Blessing|spread:Cresselia,player-1,1;Darkrai,player-1,2",
            "split|side:0",
            "heal|mon:Cresselia,player-1,1|health:205/350",
            "heal|mon:Cresselia,player-1,1|health:59/100",
            "curestatus|mon:Cresselia,player-1,1|status:Bad Poison",
            "split|side:0",
            "heal|mon:Darkrai,player-1,2|health:226/250",
            "heal|mon:Darkrai,player-1,2|health:91/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn lunar_blessing_fails_if_fully_healthy() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0;pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Cresselia,player-1,1|name:Lunar Blessing|spread:Cresselia,player-1,1;Darkrai,player-1,2",
            "split|side:0",
            "heal|mon:Cresselia,player-1,1|health:138/350",
            "heal|mon:Cresselia,player-1,1|health:40/100",
            "split|side:0",
            "heal|mon:Darkrai,player-1,2|health:138/250",
            "heal|mon:Darkrai,player-1,2|health:56/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Cresselia,player-1,1|name:Lunar Blessing|spread:Cresselia,player-1,1;Darkrai,player-1,2",
            "split|side:0",
            "heal|mon:Cresselia,player-1,1|health:226/350",
            "heal|mon:Cresselia,player-1,1|health:65/100",
            "split|side:0",
            "heal|mon:Darkrai,player-1,2|health:226/250",
            "heal|mon:Darkrai,player-1,2|health:91/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Cresselia,player-1,1|name:Lunar Blessing|spread:Cresselia,player-1,1;Darkrai,player-1,2",
            "split|side:0",
            "heal|mon:Cresselia,player-1,1|health:314/350",
            "heal|mon:Cresselia,player-1,1|health:90/100",
            "split|side:0",
            "heal|mon:Darkrai,player-1,2|health:250/250",
            "heal|mon:Darkrai,player-1,2|health:100/100",
            "residual",
            "turn|turn:4",
            "continue",
            "move|mon:Cresselia,player-1,1|name:Lunar Blessing|spread:Cresselia,player-1,1",
            "split|side:0",
            "heal|mon:Cresselia,player-1,1|health:350/350",
            "heal|mon:Cresselia,player-1,1|health:100/100",
            "fail|mon:Darkrai,player-1,2|what:heal",
            "residual",
            "turn|turn:5",
            "continue",
            "move|mon:Cresselia,player-1,1|name:Lunar Blessing|noanim",
            "fail|mon:Cresselia,player-1,1|what:heal",
            "fail|mon:Darkrai,player-1,2|what:heal",
            "fail|mon:Cresselia,player-1,1",
            "residual",
            "turn|turn:6"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
