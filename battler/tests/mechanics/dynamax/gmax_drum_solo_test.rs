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
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn rillaboom() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Rillaboom",
                    "species": "Rillaboom",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Absorb"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "gigantamax_factor": true
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn shedinja() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Shedinja",
                    "species": "Shedinja",
                    "gender": "M",
                    "ability": "Wonder Guard",
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
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .with_dynamax(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_base_damage_randomization(CoreBattleEngineRandomizeBaseDamage::Max)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn gmax_drum_solo_breaks_abilities() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(&data, 0, rillaboom().unwrap(), shedinja().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Rillaboom,player-1,1|species:Rillaboom-Gmax",
            "dynamax|mon:Rillaboom,player-1,1",
            "split|side:0",
            "sethp|mon:Rillaboom,player-1,1|health:240/240",
            "sethp|mon:Rillaboom,player-1,1|health:100/100",
            "move|mon:Rillaboom,player-1,1|name:G-Max Drum Solo|target:Shedinja,player-2,1",
            "resisted|mon:Shedinja,player-2,1",
            "crit|mon:Shedinja,player-2,1",
            "split|side:1",
            "damage|mon:Shedinja,player-2,1|health:0",
            "damage|mon:Shedinja,player-2,1|health:0",
            "faint|mon:Shedinja,player-2,1",
            "win|side:0"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
