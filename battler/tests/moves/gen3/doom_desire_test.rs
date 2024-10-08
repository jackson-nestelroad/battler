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

fn team() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Jirachi",
                    "species": "Jirachi",
                    "ability": "No Ability",
                    "moves": [
                        "Doom Desire"
                    ],
                    "nature": "Hardy",
                    "level": 50
                },
                {
                    "name": "Blaziken",
                    "species": "Blaziken",
                    "ability": "No Ability",
                    "moves": [],
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
        .with_battle_type(BattleType::Doubles)
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
fn doom_desire_attacks_slot_three_turns_later() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, team().unwrap(), team().unwrap()).unwrap();
    assert_eq!(battle.start(), Ok(()));

    assert_eq!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_eq!(
        battle.set_player_choice("player-1", "move 0,2;pass"),
        Ok(())
    );
    assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-1", "pass;pass"), Ok(()));
    assert_eq!(battle.set_player_choice("player-2", "pass;pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Jirachi,player-1,1|name:Doom Desire|noanim",
            "start|move:Doom Desire|of:Jirachi,player-1,1",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Jirachi,player-1,1|name:Doom Desire|noanim",
            "fail|mon:Jirachi,player-1,1",
            "residual",
            "turn|turn:3",
            ["time"],
            "end|move:Doom Desire|mon:Blaziken,player-2,2|of:Jirachi,player-1,1",
            "animatemove|mon:Jirachi,player-1,1|name:Doom Desire|target:Blaziken,player-2,2",
            "resisted|mon:Blaziken,player-2,2",
            "split|side:1",
            "damage|mon:Blaziken,player-2,2|health:77/140",
            "damage|mon:Blaziken,player-2,2|health:55/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
