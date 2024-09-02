use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    common::{
        Error,
        WrapResultError,
    },
    dex::{
        DataStore,
        LocalDataStore,
    },
    teams::TeamData,
};
use battler_test_utils::{
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn snorlax() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "ability": "No Ability",
                    "moves": [
                        "Revenge",
                        "Recover",
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
fn revenge_doubles_power_if_hit_by_target_this_turn() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, snorlax().unwrap(), snorlax().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 2"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Snorlax,player-1,1|name:Revenge|target:Snorlax,player-2,1",
            "supereffective|mon:Snorlax,player-2,1",
            "split|side:1",
            "damage|mon:Snorlax,player-2,1|health:134/220",
            "damage|mon:Snorlax,player-2,1|health:61/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Snorlax,player-2,1|name:Recover|target:Snorlax,player-2,1",
            "split|side:1",
            "heal|mon:Snorlax,player-2,1|health:220/220",
            "heal|mon:Snorlax,player-2,1|health:100/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Snorlax,player-2,1|name:Tackle|target:Snorlax,player-1,1",
            "split|side:0",
            "damage|mon:Snorlax,player-1,1|health:180/220",
            "damage|mon:Snorlax,player-1,1|health:82/100",
            "move|mon:Snorlax,player-1,1|name:Revenge|target:Snorlax,player-2,1",
            "supereffective|mon:Snorlax,player-2,1",
            "split|side:1",
            "damage|mon:Snorlax,player-2,1|health:72/220",
            "damage|mon:Snorlax,player-2,1|health:33/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
