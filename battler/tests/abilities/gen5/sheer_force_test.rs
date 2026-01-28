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

fn conkeldurr() -> Result<TeamData> {
    serde_json::from_str(
        r#"{
            "members": [
                {
                    "name": "Conkeldurr",
                    "species": "Conkeldurr",
                    "ability": "Sheer Force",
                    "moves": [
                        "Low Sweep",
                        "Recover"
                    ],
                    "nature": "Hardy",
                    "level": 50,
                    "item": "Life Orb"
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
fn sheer_force_cancels_secondary_effects() {
    let mut team = conkeldurr().unwrap();
    team.members[0].ability = "Mummy".to_owned();
    let mut battle = make_battle(0, conkeldurr().unwrap(), team).unwrap();
    assert_matches::assert_matches!(battle.start(), Ok(()));

    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "move 1"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-1", "move 0"), Ok(()));
    assert_matches::assert_matches!(battle.set_player_choice("player-2", "pass"), Ok(()));

    let expected_logs = serde_json::from_str::<Vec<LogMatch>>(
        r#"[
            "move|mon:Conkeldurr,player-1,1|name:Low Sweep|target:Conkeldurr,player-2,1",
            "split|side:1",
            "damage|mon:Conkeldurr,player-2,1|health:63/165",
            "damage|mon:Conkeldurr,player-2,1|health:39/100",
            "abilityend|mon:Conkeldurr,player-1,1|ability:Sheer Force|from:ability:Mummy|of:Conkeldurr,player-2,1",
            "ability|mon:Conkeldurr,player-1,1|ability:Mummy|from:ability:Mummy|of:Conkeldurr,player-2,1",
            "move|mon:Conkeldurr,player-2,1|name:Recover|target:Conkeldurr,player-2,1",
            "split|side:1",
            "heal|mon:Conkeldurr,player-2,1|health:146/165",
            "heal|mon:Conkeldurr,player-2,1|health:89/100",
            "residual",
            "turn|turn:2",
            "continue",
            "move|mon:Conkeldurr,player-1,1|name:Low Sweep|target:Conkeldurr,player-2,1",
            "split|side:1",
            "damage|mon:Conkeldurr,player-2,1|health:72/165",
            "damage|mon:Conkeldurr,player-2,1|health:44/100",
            "unboost|mon:Conkeldurr,player-2,1|stat:spe|by:1",
            "split|side:0",
            "damage|mon:Conkeldurr,player-1,1|from:item:Life Orb|health:149/165",
            "damage|mon:Conkeldurr,player-1,1|from:item:Life Orb|health:91/100",
            "residual",
            "turn|turn:3"
        ]"#,
    )
    .unwrap();
    assert_logs_since_turn_eq(&battle, 1, &expected_logs);
}
