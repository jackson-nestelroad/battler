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
    get_controlled_rng_for_battle,
};

fn linoone() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Linoone",
                    "species": "Linoone",
                    "ability": "Pickup",
                    "moves": [
                        "Thunder Wave",
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
) -> Result<PublicCoreBattle<'_>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .with_controlled_rng(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn pickup_picks_up_random_used_item() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut player = linoone().unwrap();
    player.members[0].item = Some("Rawst Berry".to_owned());
    let mut opponent = linoone().unwrap();
    opponent.members[0].item = Some("Cheri Berry".to_owned());
    let mut battle = make_battle(&data, 0, player, opponent).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 1)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    let rng = get_controlled_rng_for_battle(&mut battle).unwrap();
    rng.insert_fake_values_relative_to_sequence_count([(3, 0)]);

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Linoone,player-1,1|name:Thunder Wave|target:Linoone,player-2,1",
            "status|mon:Linoone,player-2,1|status:Paralysis",
            "itemend|mon:Linoone,player-2,1|item:Cheri Berry|eat",
            "curestatus|mon:Linoone,player-2,1|status:Paralysis|from:item:Cheri Berry",
            "move|mon:Linoone,player-2,1|name:Will-O-Wisp|target:Linoone,player-1,1",
            "status|mon:Linoone,player-1,1|status:Burn",
            "itemend|mon:Linoone,player-1,1|item:Rawst Berry|eat",
            "curestatus|mon:Linoone,player-1,1|status:Burn|from:item:Rawst Berry",
            "item|mon:Linoone,player-1,1|item:Cheri Berry|from:ability:Pickup",
            "item|mon:Linoone,player-2,1|item:Rawst Berry|from:ability:Pickup",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Linoone,player-1,1|name:Will-O-Wisp|target:Linoone,player-2,1",
            "status|mon:Linoone,player-2,1|status:Burn",
            "itemend|mon:Linoone,player-2,1|item:Rawst Berry|eat",
            "curestatus|mon:Linoone,player-2,1|status:Burn|from:item:Rawst Berry",
            "move|mon:Linoone,player-2,1|name:Thunder Wave|target:Linoone,player-1,1",
            "status|mon:Linoone,player-1,1|status:Paralysis",
            "itemend|mon:Linoone,player-1,1|item:Cheri Berry|eat",
            "curestatus|mon:Linoone,player-1,1|status:Paralysis|from:item:Cheri Berry",
            "item|mon:Linoone,player-1,1|item:Cheri Berry|from:ability:Pickup",
            "item|mon:Linoone,player-2,1|item:Rawst Berry|from:ability:Pickup",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
