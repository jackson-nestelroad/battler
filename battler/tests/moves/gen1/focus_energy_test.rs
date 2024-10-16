use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    error::{
        Error,
        WrapResultError,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Nidoking",
                    "species": "Nidoking",
                    "ability": "No Ability",
                    "moves": [
                        "Focus Energy",
                        "Spike Cannon"
                    ],
                    "nature": "Hardy",
                    "gender": "M",
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
) -> Result<PublicCoreBattle, Error> {
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
fn focus_energy_increases_crit_ratio() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 425479950183495, team().unwrap(), team().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Nidoking,player-1,1|name:Focus Energy|target:Nidoking,player-1,1",
            "start|mon:Nidoking,player-1,1|move:Focus Energy",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
            "crit|mon:Nidoking,player-2,1",
            "split|side:1",
            "damage|mon:Nidoking,player-2,1|health:124/141",
            "damage|mon:Nidoking,player-2,1|health:88/100",
            "animatemove|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
            "crit|mon:Nidoking,player-2,1",
            "split|side:1",
            "damage|mon:Nidoking,player-2,1|health:108/141",
            "damage|mon:Nidoking,player-2,1|health:77/100",
            "animatemove|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
            "crit|mon:Nidoking,player-2,1",
            "split|side:1",
            "damage|mon:Nidoking,player-2,1|health:92/141",
            "damage|mon:Nidoking,player-2,1|health:66/100",
            "animatemove|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
            "crit|mon:Nidoking,player-2,1",
            "split|side:1",
            "damage|mon:Nidoking,player-2,1|health:74/141",
            "damage|mon:Nidoking,player-2,1|health:53/100",
            "animatemove|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
            "crit|mon:Nidoking,player-2,1",
            "split|side:1",
            "damage|mon:Nidoking,player-2,1|health:56/141",
            "damage|mon:Nidoking,player-2,1|health:40/100",
            "hitcount|hits:5",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
            "crit|mon:Nidoking,player-2,1",
            "split|side:1",
            "damage|mon:Nidoking,player-2,1|health:40/141",
            "damage|mon:Nidoking,player-2,1|health:29/100",
            "animatemove|mon:Nidoking,player-1,1|name:Spike Cannon|target:Nidoking,player-2,1",
            "split|side:1",
            "damage|mon:Nidoking,player-2,1|health:28/141",
            "damage|mon:Nidoking,player-2,1|health:20/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
