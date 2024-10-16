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

fn meloetta() -> Result<TeamData, Error> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Meloetta",
                    "species": "Meloetta",
                    "ability": "No Ability",
                    "moves": [
                        "Flamethrower"
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
fn fire_gem_boosts_fire_move_base_power() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = meloetta().unwrap();
    team.members[0].item = Some("Fire Gem".to_owned());
    let mut battle = make_battle(&data, 0, team, meloetta().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Meloetta,player-1,1|name:Flamethrower|target:Meloetta,player-2,1",
            "itemend|mon:Meloetta,player-1,1|item:Fire Gem",
            "split|side:1",
            "damage|mon:Meloetta,player-2,1|health:109/160",
            "damage|mon:Meloetta,player-2,1|health:69/100",
            "move|mon:Meloetta,player-2,1|name:Flamethrower|target:Meloetta,player-1,1",
            "split|side:0",
            "damage|mon:Meloetta,player-1,1|health:123/160",
            "damage|mon:Meloetta,player-1,1|health:77/100",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
