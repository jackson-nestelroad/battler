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

fn sableye() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Sableye",
                    "species": "Sableye",
                    "ability": "No Ability",
                    "moves": [
                        "Knock Off",
                        "Will-O-Wisp"
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
fn knock_off_increases_power_against_target_with_item() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut target = sableye().unwrap();
    target.members[0].item = Some("Rawst Berry".to_owned());
    let mut battle = make_battle(&data, 0, sableye().unwrap(), target).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Sableye,player-1,1|name:Knock Off|target:Sableye,player-2,1",
            "split|side:1",
            "damage|mon:Sableye,player-2,1|health:47/110",
            "damage|mon:Sableye,player-2,1|health:43/100",
            "itemend|mon:Sableye,player-2,1|item:Rawst Berry|from:move:Knock Off|of:Sableye,player-1,1",
            "move|mon:Sableye,player-2,1|name:Knock Off|target:Sableye,player-1,1",
            "split|side:0",
            "damage|mon:Sableye,player-1,1|health:70/110",
            "damage|mon:Sableye,player-1,1|health:64/100",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Sableye,player-1,1|name:Will-O-Wisp|target:Sableye,player-2,1",
            "status|mon:Sableye,player-2,1|status:Burn",
            "split|side:1",
            "damage|mon:Sableye,player-2,1|from:status:Burn|health:41/110",
            "damage|mon:Sableye,player-2,1|from:status:Burn|health:38/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
