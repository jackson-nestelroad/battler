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
                    "name": "Patrat",
                    "species": "Patrat",
                    "ability": "No Ability",
                    "moves": [
                        "After You",
                        "Tackle"
                    ],
                    "nature": "Timid",
                    "gender": "M",
                    "level": 50,
                    "evs": {
                        "spe": 252
                    }
                },
                {
                    "name": "Patrat",
                    "species": "Patrat",
                    "ability": "No Ability",
                    "moves": [
                        "After You",
                        "Tackle"
                    ],
                    "nature": "Brave",
                    "gender": "M",
                    "level": 50,
                    "ivs": {
                        "spe": 0
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn after_you_moves_target_next() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,-2;move 1,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 1,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Patrat,player-1,1|name:After You|target:Patrat,player-1,2",
            "activate|mon:Patrat,player-1,2|move:After You",
            "move|mon:Patrat,player-1,2|name:Tackle|target:Patrat,player-2,1",
            "split|side:1",
            "damage|mon:Patrat,player-2,1|health:65/105",
            "damage|mon:Patrat,player-2,1|health:62/100",
            "move|mon:Patrat,player-2,1|name:Tackle|target:Patrat,player-1,1",
            "split|side:0",
            "damage|mon:Patrat,player-1,1|health:75/105",
            "damage|mon:Patrat,player-1,1|health:72/100",
            "move|mon:Patrat,player-2,2|name:Tackle|target:Patrat,player-1,1",
            "split|side:0",
            "damage|mon:Patrat,player-1,1|health:41/105",
            "damage|mon:Patrat,player-1,1|health:40/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn after_you_fails_if_target_already_moved() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 1,1;move 0,-1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 1,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Patrat,player-1,1|name:Tackle|target:Patrat,player-2,1",
            "split|side:1",
            "damage|mon:Patrat,player-2,1|health:72/105",
            "damage|mon:Patrat,player-2,1|health:69/100",
            "move|mon:Patrat,player-2,1|name:Tackle|target:Patrat,player-1,1",
            "split|side:0",
            "damage|mon:Patrat,player-1,1|health:75/105",
            "damage|mon:Patrat,player-1,1|health:72/100",
            "move|mon:Patrat,player-1,2|name:After You|noanim",
            "fail|mon:Patrat,player-1,2",
            "move|mon:Patrat,player-2,2|name:Tackle|target:Patrat,player-1,1",
            "split|side:0",
            "damage|mon:Patrat,player-1,1|health:41/105",
            "damage|mon:Patrat,player-1,1|health:40/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn after_you_works_on_opponent() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "move 0,2;move 1,1"),
        Ok(())
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1,1;move 1,1"),
        Ok(())
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Patrat,player-1,1|name:After You|target:Patrat,player-2,2",
            "activate|mon:Patrat,player-2,2|move:After You",
            "move|mon:Patrat,player-2,2|name:Tackle|target:Patrat,player-1,1",
            "split|side:0",
            "damage|mon:Patrat,player-1,1|health:65/105",
            "damage|mon:Patrat,player-1,1|health:62/100",
            "move|mon:Patrat,player-2,1|name:Tackle|target:Patrat,player-1,1",
            "split|side:0",
            "damage|mon:Patrat,player-1,1|health:35/105",
            "damage|mon:Patrat,player-1,1|health:34/100",
            "move|mon:Patrat,player-1,2|name:Tackle|target:Patrat,player-2,1",
            "split|side:1",
            "damage|mon:Patrat,player-2,1|health:71/105",
            "damage|mon:Patrat,player-2,1|health:68/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
