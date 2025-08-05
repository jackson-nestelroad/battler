use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineRandomizeBaseDamage,
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

fn snorlax() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Snorlax",
                    "species": "Snorlax",
                    "ability": "Gluttony",
                    "moves": [
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 40,
                    "item": "Custap Berry"
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn jolteon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Jolteon",
                    "species": "Jolteon",
                    "ability": "No Ability",
                    "moves": [
                        "Close Combat"
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
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Min)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn custap_berry_allows_user_to_move_first() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, snorlax().unwrap(), jolteon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Jolteon,player-2,1|name:Close Combat|target:Snorlax,player-1,1",
            "supereffective|mon:Snorlax,player-1,1",
            "split|side:0",
            "damage|mon:Snorlax,player-1,1|health:66/178",
            "damage|mon:Snorlax,player-1,1|health:38/100",
            "unboost|mon:Jolteon,player-2,1|stat:def|by:1",
            "unboost|mon:Jolteon,player-2,1|stat:spa|by:1",
            "residual",
            "turn|turn:2",
            "itemend|mon:Snorlax,player-1,1|item:Custap Berry|eat",
            "activate|mon:Snorlax,player-1,1|item:Custap Berry",
            ["time"],
            "move|mon:Snorlax,player-1,1|name:Earthquake",
            "supereffective|mon:Jolteon,player-2,1",
            "split|side:1",
            "damage|mon:Jolteon,player-2,1|health:0",
            "damage|mon:Jolteon,player-2,1|health:0",
            "faint|mon:Jolteon,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
