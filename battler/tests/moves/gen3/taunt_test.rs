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
    assert_error_message,
    assert_logs_since_turn_eq,
    LogMatch,
    TestBattleBuilder,
};

fn treecko() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Treecko",
                    "species": "Treecko",
                    "ability": "No Ability",
                    "moves": [
                        "Taunt",
                        "Tackle",
                        "Trick"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Reverse)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn taunt_disables_status_moves() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 100, treecko().unwrap(), treecko().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    assert_error_message(
        battle.set_player_choice("player-1", "move 0"),
        "cannot move: Treecko's Taunt is disabled",
    );
    assert_error_message(
        battle.set_player_choice("player-1", "move 2"),
        "cannot move: Treecko's Trick is disabled",
    );

    assert_eq!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Treecko,player-2,1|name:Taunt|target:Treecko,player-1,1",
            "start|mon:Treecko,player-1,1|move:Taunt",
            "cant|mon:Treecko,player-1,1|reason:move:Taunt",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Treecko,player-2,1|name:Taunt|noanim",
            "fail|mon:Treecko,player-2,1",
            "move|mon:Treecko,player-1,1|name:Tackle|target:Treecko,player-2,1",
            "split|side:1",
            "damage|mon:Treecko,player-2,1|health:78/100",
            "damage|mon:Treecko,player-2,1|health:78/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
