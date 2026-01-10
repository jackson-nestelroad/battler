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

fn scolipede() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Scolipede",
                    "species": "Scolipede",
                    "ability": "No Ability",
                    "moves": [
                        "Autotomize",
                        "Grass Knot"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn make_battle(seed: u64, team1: TeamData, team2: TeamData) -> Result<PublicCoreBattle<'static>> {
    TestBattleBuilder::new()
        .with_battle_type(BattleType::Singles)
        .with_seed(seed)
        .with_team("player-1", team1)
        .with_team("player-2", team2)
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .with_team_validation(false)
        .with_pass_allowed(true)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .build(static_local_data_store())
}

#[test]
fn autotomize_boosts_speed_and_reduces_weight() {
    let mut battle = make_battle(0, scolipede().unwrap(), scolipede().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Scolipede,player-1,1|name:Grass Knot|target:Scolipede,player-2,1",
            "resisted|mon:Scolipede,player-2,1",
            "split|side:1",
            "damage|mon:Scolipede,player-2,1|health:110/120",
            "damage|mon:Scolipede,player-2,1|health:92/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Scolipede,player-1,1|name:Autotomize|target:Scolipede,player-1,1",
            "boost|mon:Scolipede,player-1,1|stat:spe|by:2",
            "start|mon:Scolipede,player-1,1|move:Autotomize",
            "residual",
            "turn|turn:3",
            "continue",
            "move|mon:Scolipede,player-1,1|name:Grass Knot|target:Scolipede,player-2,1",
            "resisted|mon:Scolipede,player-2,1",
            "split|side:1",
            "damage|mon:Scolipede,player-2,1|health:101/120",
            "damage|mon:Scolipede,player-2,1|health:85/100",
            "residual",
            "turn|turn:4"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
