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

fn corviknight() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Corviknight",
                    "species": "Corviknight",
                    "gender": "M",
                    "ability": "No Ability",
                    "moves": [
                        "Fly",
                        "Spikes",
                        "Toxic Spikes",
                        "Light Screen",
                        "Mist",
                        "Psychic Terrain",
                        "Substitute"
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
fn gmax_wind_rage_clears_target_side_conditions_and_hazards() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle =
        make_battle(&data, 100, corviknight().unwrap(), corviknight().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 3"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 4"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 5"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 6"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,dyna"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "gigantamax|mon:Corviknight,player-1,1|species:Corviknight-Gmax",
            "dynamax|mon:Corviknight,player-1,1",
            "split|side:0",
            "sethp|mon:Corviknight,player-1,1|health:237/237",
            "sethp|mon:Corviknight,player-1,1|health:100/100",
            "move|mon:Corviknight,player-1,1|name:G-Max Wind Rage|target:Corviknight,player-2,1",
            "resisted|mon:Corviknight,player-2,1",
            "activate|mon:Corviknight,player-2,1|move:Substitute|damage",
            "sideend|side:1|move:Light Screen",
            "sideend|side:1|move:Mist",
            "sideend|side:0|move:Spikes",
            "sideend|side:1|move:Spikes",
            "sideend|side:0|move:Toxic Spikes",
            "sideend|side:1|move:Toxic Spikes",
            "fieldend|move:Psychic Terrain",
            "residual",
            "turn|turn:8"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 7, &expected_logs);
}
