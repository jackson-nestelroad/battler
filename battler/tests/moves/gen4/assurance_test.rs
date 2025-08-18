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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn rampardos() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Drifblim",
                    "species": "Drifblim",
                    "ability": "No Ability",
                    "item": "Life Orb",
                    "moves": [
                        "Assurance"
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn assurance_doubles_power_after_target_takes_damage() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, rampardos().unwrap(), rampardos().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Drifblim,player-1,1|name:Assurance|target:Drifblim,player-2,1",
            "supereffective|mon:Drifblim,player-2,1",
            "split|side:1",
            "damage|mon:Drifblim,player-2,1|health:93/210",
            "damage|mon:Drifblim,player-2,1|health:45/100",
            "split|side:0",
            "damage|mon:Drifblim,player-1,1|from:item:Life Orb|health:189/210",
            "damage|mon:Drifblim,player-1,1|from:item:Life Orb|health:90/100",
            "move|mon:Drifblim,player-2,1|name:Assurance|target:Drifblim,player-1,1",
            "supereffective|mon:Drifblim,player-1,1",
            "split|side:0",
            "damage|mon:Drifblim,player-1,1|health:0",
            "damage|mon:Drifblim,player-1,1|health:0",
            "faint|mon:Drifblim,player-1,1",
            "split|side:1",
            "damage|mon:Drifblim,player-2,1|from:item:Life Orb|health:72/210",
            "damage|mon:Drifblim,player-2,1|from:item:Life Orb|health:35/100",
            "win|side:1" 
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
