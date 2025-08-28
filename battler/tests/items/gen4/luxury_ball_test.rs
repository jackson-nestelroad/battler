use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    TeamData,
    WildPlayerOptions,
    WrapResultError,
    battle::PlayerDex,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn piplup() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Piplup",
                    "species": "Piplup",
                    "ability": "No Ability",
                    "moves": [],
                    "nature": "Hardy",
                    "level": 50,
                    "ball": "Luxury Ball"
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
    dex: PlayerDex,
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_bag_items(true)
        .with_infinite_bags(true)
        .add_protagonist_to_side_1("protagonist", "Protagonist")
        .add_wild_mon_to_side_2("wild", "Wild", WildPlayerOptions::default())
        .with_team("protagonist", team_1)
        .with_player_dex("protagonist", dex)
        .with_team("wild", team_2)
        .build(data)
}

#[test]
fn luxury_ball_increases_friendship_gained() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut battle = make_battle(
        &data,
        0,
        piplup().unwrap(),
        piplup().unwrap(),
        PlayerDex::default(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(
        battle.set_player_choice("protagonist", "item guardspec"),
        Ok(())
    );
    assert_matches::assert_matches!(battle.set_player_choice("wild", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "useitem|player:protagonist|name:Guard Spec.|target:Piplup,protagonist,1",
            "sidestart|side:0|move:Mist",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);

    assert_matches::assert_matches!(battle.player_data("protagonist"), Ok(data) => {
        assert_eq!(data.mons[0].summary.friendship, 2);
    });
}
