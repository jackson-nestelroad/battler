use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    DataStore,
    LocalDataStore,
    PublicCoreBattle,
    Request,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
};

fn rayquaza() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Rayquaza",
                    "species": "Rayquaza",
                    "ability": "No Ability",
                    "moves": [
                        "Splash",
                        "Dragon Ascent"
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
        .with_mega_evolution(true)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(data)
}

#[test]
fn rayquaza_mega_evolves_when_knowing_dragon_ascent() {
    let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
    let mut team = rayquaza().unwrap();
    team.members[0].moves.pop();
    let mut battle = make_battle(&data, 0, rayquaza().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.request_for_player("player-1"), Ok(Some(Request::Turn(request))) => {
        assert!(request.active[0].can_mega_evolve);
    });

    assert_matches::assert_matches!(battle.request_for_player("player-2"), Ok(Some(Request::Turn(request))) => {
        assert!(!request.active[0].can_mega_evolve);
    });

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Rayquaza-Mega"],
            ["specieschange", "player-1", "species:Rayquaza-Mega"],
            "mega|mon:Rayquaza,player-1,1|species:Rayquaza-Mega|from:move:Dragon Ascent",
            "weather|weather:Strong Winds|from:ability:Delta Stream|of:Rayquaza,player-1,1",
            "move|mon:Rayquaza,player-1,1|name:Splash|target:Rayquaza,player-1,1",
            "activate|move:Splash",
            "weather|weather:Strong Winds|residual",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
