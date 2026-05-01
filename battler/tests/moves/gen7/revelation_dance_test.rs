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
                    "name": "Oricorio",
                    "species": "Oricorio",
                    "ability": "No Ability",
                    "moves": [
                        "Revelation Dance",
                        "Burn Up",
                        "Soak"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn revelation_dance_matches_first_type_of_target() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Oricorio,player-1,1|name:Revelation Dance|target:Oricorio,player-2,1",
            "resisted|mon:Oricorio,player-2,1",
            "split|side:1",
            "damage|mon:Oricorio,player-2,1|health:184/260",
            "damage|mon:Oricorio,player-2,1|health:71/100",
            "move|mon:Oricorio,player-2,1|name:Burn Up|target:Oricorio,player-1,1",
            "resisted|mon:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:158/260",
            "damage|mon:Oricorio,player-1,1|health:61/100",
            "typechange|mon:Oricorio,player-2,1|types:Flying",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Oricorio,player-2,1|name:Revelation Dance|target:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:23/260",
            "damage|mon:Oricorio,player-1,1|health:9/100",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Oricorio,player-1,1|name:Soak|target:Oricorio,player-2,1",
            "typechange|mon:Oricorio,player-2,1|types:Water",
            "move|mon:Oricorio,player-2,1|name:Revelation Dance|target:Oricorio,player-1,1",
            "supereffective|mon:Oricorio,player-1,1",
            "split|side:0",
            "damage|mon:Oricorio,player-1,1|health:0",
            "damage|mon:Oricorio,player-1,1|health:0",
            "faint|mon:Oricorio,player-1,1",
            "win|side:1" 
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
