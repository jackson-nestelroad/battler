use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineSpeedSortTieResolution,
    PublicCoreBattle,
    TeamData,
    WrapResultError,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};

fn kangaskhan() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Kangaskhan",
                    "species": "Kangaskhan",
                    "ability": "Early Bird",
                    "item": "Kangaskhanite",
                    "moves": [
                        "Tackle",
                        "Earthquake"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team_1: TeamData, team_2: TeamData) -> Result<PublicCoreBattle<'static>> {
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
        .build(static_local_data_store())
}

#[test]
fn kangaskhan_mega_evolves_and_gains_parental_bond() {
    let mut battle = make_battle(
        0,
        kangaskhan().unwrap(),
        kangaskhan().unwrap(),
    )
    .unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,mega"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "split|side:0",
            ["specieschange", "player-1", "species:Kangaskhan-Mega"],
            ["specieschange", "player-1", "species:Kangaskhan-Mega"],
            "mega|mon:Kangaskhan,player-1,1|species:Kangaskhan-Mega|from:item:Kangaskhanite",
            "move|mon:Kangaskhan,player-1,1|name:Tackle|target:Kangaskhan,player-2,1",
            "split|side:1",
            "damage|mon:Kangaskhan,player-2,1|health:125/165",
            "damage|mon:Kangaskhan,player-2,1|health:76/100",
            "animatemove|mon:Kangaskhan,player-1,1|name:Tackle|target:Kangaskhan,player-2,1",
            "split|side:1",
            "damage|mon:Kangaskhan,player-2,1|health:115/165",
            "damage|mon:Kangaskhan,player-2,1|health:70/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:2",
            ["time"],
            "move|mon:Kangaskhan,player-1,1|name:Earthquake",
            "split|side:1",
            "damage|mon:Kangaskhan,player-2,1|health:50/165",
            "damage|mon:Kangaskhan,player-2,1|health:31/100",
            "animatemove|mon:Kangaskhan,player-1,1|name:Earthquake",
            "split|side:1",
            "damage|mon:Kangaskhan,player-2,1|health:33/165",
            "damage|mon:Kangaskhan,player-2,1|health:20/100",
            "hitcount|hits:2",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
