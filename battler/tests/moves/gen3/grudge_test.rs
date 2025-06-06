use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Misdreavus",
                    "species": "Misdreavus",
                    "ability": "No Ability",
                    "moves": [
                        "Grudge",
                        "Dark Pulse"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Misdreavus",
                    "species": "Misdreavus",
                    "ability": "No Ability",
                    "moves": [
                        "Grudge",
                        "Dark Pulse"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(
    data: &dyn DataStore,
    seed: u64,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle> {
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
        .build(data)
}

#[test]
fn grudge_sets_last_move_pp_to_zero_on_faint() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "switch 1"), Ok(()));
    assert_matches::assert_matches!(
        battle.set_player_choice("player-2", "move 1"),
        Err(err) => assert_eq!(format!("{err:#}"), "invalid choice 0: cannot move: Misdreavus's Dark Pulse is disabled")
    );

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Misdreavus,player-1,1|name:Grudge|target:Misdreavus,player-1,1",
            "singlemove|mon:Misdreavus,player-1,1|move:Grudge",
            "move|mon:Misdreavus,player-2,1|name:Dark Pulse|target:Misdreavus,player-1,1",
            "supereffective|mon:Misdreavus,player-1,1",
            "split|side:0",
            "damage|mon:Misdreavus,player-1,1|health:0",
            "damage|mon:Misdreavus,player-1,1|health:0",
            "faint|mon:Misdreavus,player-1,1",
            "setpp|mon:Misdreavus,player-2,1|move:Dark Pulse|to:0|from:move:Grudge|of:Misdreavus,player-1,1",
            "residual",
            ["time"],
            "split|side:0",
            ["switch", "player-1", "Misdreavus"],
            ["switch", "player-1", "Misdreavus"],
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 2, &expected_logs);
}
