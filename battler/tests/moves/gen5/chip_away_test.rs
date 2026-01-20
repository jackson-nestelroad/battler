use anyhow::Result;
use battler::{
    battle::{
        BattleType,
        CoreBattleEngineSpeedSortTieResolution,
        PublicCoreBattle,
    },
    teams::TeamData,
};
use battler_test_utils::{
    LogMatch,
    TestBattleBuilder,
    assert_logs_since_turn_eq,
    static_local_data_store,
};
use serde_json;

fn team() -> TeamData {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Conkeldurr",
                    "species": "Conkeldurr",
                    "moves": ["Chip Away", "Iron Defense", "Minimize"],
                    "level": 50,
                    "nature": "Hardy",
                    "ability": "No Ability"
                }
            ]
        }"#,
    )
    .unwrap()
}

fn make_battle(
    battle_type: BattleType,
    team_1: TeamData,
    team_2: TeamData,
) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(battle_type)
        .with_seed(0)
        .with_team_validation(false)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_pass_allowed(true)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn chip_away_ignores_defensive_and_evasion_boosts() {
    let mut battle = make_battle(BattleType::Singles, team(), team()).unwrap();

    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 2"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0,1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0,1"), Ok(()));

    let expected_logs: Vec<LogMatch> = serde_json::from_str(
        r#"[
            "move|mon:Conkeldurr,player-1,1|name:Chip Away|target:Conkeldurr,player-2,1",
            "split|side:1",
            "damage|mon:Conkeldurr,player-2,1|health:121/165",
            "damage|mon:Conkeldurr,player-2,1|health:74/100",
            "move|mon:Conkeldurr,player-2,1|name:Chip Away|target:Conkeldurr,player-1,1",
            "split|side:0",
            "damage|mon:Conkeldurr,player-1,1|health:124/165",
            "damage|mon:Conkeldurr,player-1,1|health:76/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Conkeldurr,player-1,1|name:Minimize|target:Conkeldurr,player-1,1",
            "boost|mon:Conkeldurr,player-1,1|stat:eva|by:2",
            "move|mon:Conkeldurr,player-2,1|name:Iron Defense|target:Conkeldurr,player-2,1",
            "boost|mon:Conkeldurr,player-2,1|stat:def|by:2",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Conkeldurr,player-1,1|name:Chip Away|target:Conkeldurr,player-2,1",
            "split|side:1",
            "damage|mon:Conkeldurr,player-2,1|health:82/165",
            "damage|mon:Conkeldurr,player-2,1|health:50/100",
            "move|mon:Conkeldurr,player-2,1|name:Chip Away|target:Conkeldurr,player-1,1",
            "split|side:0",
            "damage|mon:Conkeldurr,player-1,1|health:83/165",
            "damage|mon:Conkeldurr,player-1,1|health:51/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
