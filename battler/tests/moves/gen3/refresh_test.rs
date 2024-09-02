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

fn swablu() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Swablu",
                    "species": "Swablu",
                    "ability": "No Ability",
                    "moves": [
                        "Refresh",
                        "Toxic"
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
fn refresh_heals_user_status() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, swablu().unwrap(), swablu().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Swablu,player-1,1|name:Refresh|noanim",
            "fail|mon:Swablu,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Swablu,player-2,1|name:Toxic|target:Swablu,player-1,1",
            "status|mon:Swablu,player-1,1|status:Bad Poison",
            "split|side:0",
            "damage|mon:Swablu,player-1,1|from:status:Bad Poison|health:99/105",
            "damage|mon:Swablu,player-1,1|from:status:Bad Poison|health:95/100",
            "residual",
            "turn|turn:3",
            ["time"],
            "move|mon:Swablu,player-1,1|name:Refresh|target:Swablu,player-1,1",
            "curestatus|mon:Swablu,player-1,1|status:Bad Poison",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}