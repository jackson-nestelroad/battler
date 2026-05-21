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
                    "name": "Drednaw",
                    "species": "Drednaw",
                    "ability": "Emergency Exit",
                    "moves": [
                        "Jaw Lock",
                        "Thunderbolt"
                    ],
                    "nature": "Hardy",
                    "level": 100
                },
                {
                    "name": "Drednaw",
                    "species": "Drednaw",
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
fn jaw_lock_traps_user_and_target() {
    let mut battle = make_battle(0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("player-1", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: Drednaw is trapped")
    );
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "switch 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot switch: Drednaw is trapped", "{err:?}")
    );

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "switch 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Drednaw,player-1,1|name:Jaw Lock|target:Drednaw,player-2,1",
            "split|side:1",
            "damage|mon:Drednaw,player-2,1|health:206/290",
            "damage|mon:Drednaw,player-2,1|health:72/100",
            "activate|mon:Drednaw,player-2,1|condition:Trapped",
            "activate|mon:Drednaw,player-1,1|condition:Trapped",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Drednaw,player-1,1|name:Thunderbolt|target:Drednaw,player-2,1",
            "supereffective|mon:Drednaw,player-2,1",
            "split|side:1",
            "damage|mon:Drednaw,player-2,1|health:106/290",
            "damage|mon:Drednaw,player-2,1|health:37/100",
            "activate|mon:Drednaw,player-2,1|ability:Emergency Exit",
            "switchout|mon:Drednaw,player-2,1",
            "continue",
            "split|side:1",
            ["switch", "player-2", "Drednaw"],
            ["switch", "player-2", "Drednaw"],
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
