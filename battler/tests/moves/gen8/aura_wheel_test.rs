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

fn morpeko() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Morpeko",
                    "species": "Morpeko",
                    "ability": "Hunger Switch",
                    "moves": [
                        "Aura Wheel"
                    ],
                    "nature": "Hardy",
                    "level": 50
                }
            ]
        }"#,
    )
    .wrap_error()
}

fn kecleon() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Kecleon",
                    "species": "Kecleon",
                    "ability": "Color Change",
                    "moves": [
                        "Aura Wheel"
                    ],
                    "nature": "Hardy",
                    "level": 100
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
        .with_speed_sort_tie_resolution(CoreBattleEngineSpeedSortTieResolution::Keep)
        .add_player_to_side_1("player-1", "Player 1")
        .add_player_to_side_2("player-2", "Player 2")
        .with_team("player-1", team_1)
        .with_team("player-2", team_2)
        .build(static_local_data_store())
}

#[test]
fn aura_wheel_changes_type_based_on_morpeko_forme() {
    let mut battle = make_battle(0, morpeko().unwrap(), kecleon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Morpeko,player-1,1|name:Aura Wheel|target:Kecleon,player-2,1",
            "split|side:1",
            "damage|mon:Kecleon,player-2,1|health:181/230",
            "damage|mon:Kecleon,player-2,1|health:79/100",
            "boost|mon:Kecleon,player-2,1|stat:spe|by:1",
            "typechange|mon:Kecleon,player-2,1|types:Electric",
            "formechange|mon:Morpeko,player-1,1|species:Morpeko-Hangry|from:ability:Hunger Switch",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Morpeko,player-1,1|name:Aura Wheel|target:Kecleon,player-2,1",
            "split|side:1",
            "damage|mon:Kecleon,player-2,1|health:135/230",
            "damage|mon:Kecleon,player-2,1|health:59/100",
            "boost|mon:Kecleon,player-2,1|stat:spe|by:1",
            "typechange|mon:Kecleon,player-2,1|types:Dark",
            "formechange|mon:Morpeko,player-1,1|species:Morpeko|from:ability:Hunger Switch",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}

#[test]
fn aura_wheel_cannot_be_used_by_non_morpeko() {
    let mut battle = make_battle(0, morpeko().unwrap(), kecleon().unwrap()).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "pass"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 0"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Kecleon,player-2,1|name:Aura Wheel|noanim",
            "cant|mon:Kecleon,player-2,1|from:move:Aura Wheel",
            "formechange|mon:Morpeko,player-1,1|species:Morpeko-Hangry|from:ability:Hunger Switch",
            "residual",
            "turn|turn:2"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
