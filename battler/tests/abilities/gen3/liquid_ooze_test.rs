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

fn swalot() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Swalot",
                    "species": "Swalot",
                    "ability": "Liquid Ooze",
                    "moves": [
                        "Giga Drain",
                        "Mud Shot"
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
fn liquid_ooze_damages_attacker_on_drain() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, swalot().unwrap(), swalot().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swalot,player-2,1|name:Mud Shot|target:Swalot,player-1,1",
            "supereffective|mon:Swalot,player-1,1",
            "split|side:0",
            "damage|mon:Swalot,player-1,1|health:116/160",
            "damage|mon:Swalot,player-1,1|health:73/100",
            "unboost|mon:Swalot,player-1,1|stat:spe|by:1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Swalot,player-1,1|name:Giga Drain|target:Swalot,player-2,1",
            "resisted|mon:Swalot,player-2,1",
            "split|side:1",
            "damage|mon:Swalot,player-2,1|health:147/160",
            "damage|mon:Swalot,player-2,1|health:92/100",
            "split|side:0",
            "damage|mon:Swalot,player-1,1|from:ability:Liquid Ooze|of:Swalot,player-2,1|health:109/160",
            "damage|mon:Swalot,player-1,1|from:ability:Liquid Ooze|of:Swalot,player-2,1|health:69/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
